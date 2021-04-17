pub mod network;
mod broadcast;
mod command;
use crate::miner;
use crate::block::{Block, BlockChain};
use crate::wallet::PrivateWallet;
use crate::logger::{Logger, LoggerLevel};
use network::{NetworkConnection, Packet};
use command::{Command, TransactionCommand, PageCommand, BalanceCommand};

use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Mutex, Arc};
use std::path::PathBuf;
use std::time::Duration;
use std::io::Write;

pub struct Node<W: Write>
{
    connection: Arc<Mutex<NetworkConnection>>,
    blocks_being_mined: i32,

    commands: Vec<Box<dyn Command<W>>>,
}

impl<W: Write> Node<W>
{

    pub fn new(port: i32, known_nodes: PathBuf) -> Self
    {
        let connection = NetworkConnection::new(port, known_nodes).unwrap();
        Self
        {
            connection: Arc::from(Mutex::from(connection)),
            blocks_being_mined: 0,
            
            commands: vec![
                Box::from(TransactionCommand::default()),
                Box::from(BalanceCommand::default()),
                Box::from(PageCommand::default()),
            ],
        }
    }

    pub fn add_known_node(&mut self, address: &str)
    {
        self.connection.lock().unwrap().update_known_nodes(address);
    }

    fn miner_thread(to_mine: Receiver<Block>, done: Sender<Block>)
    {
        for block in to_mine {
            done.send(miner::mine_block(block)).expect("worked");
        }
    }

    fn process_received_block(&mut self, chain: &mut BlockChain, block: Block, logger: &mut Logger<W>)
    {
        logger.log(LoggerLevel::Verbose, &format!("Got block {}", block.block_id));
        chain.add(block.clone(), logger);
    }

    fn process_block_request(&mut self, chain: &mut BlockChain, address: String, block_id: u64, logger: &mut Logger<W>)
    {
        logger.log(LoggerLevel::Verbose, &format!("Got request from {} for {}", address, block_id));
        if block_id > chain.top_id() {
            return;
        }

        let block = chain.block(block_id);
        if block.is_some() 
        {
            NetworkConnection::broadcast(&mut self.connection, 
                Some( address ), Packet::NewBlock(block.unwrap()));
        }
    }

    fn process_packets(&mut self, chain: &mut BlockChain, logger: &mut Logger<W>)
    {
        for packet in NetworkConnection::process_packets(&mut self.connection)
        {
            match packet
            {
                Packet::Hello(address) => self.process_block_request(chain, address, chain.top_id(), logger),
                Packet::NewBlock(block) => self.process_received_block(chain, block, logger),
                Packet::BlockRequest(address, block_id) => self.process_block_request(chain, address, block_id, logger),

                _ =>
                {
                    for command in &mut self.commands {
                        command.on_packet(packet.clone(), &mut self.connection, chain);
                    }
                },
            }
        }
    }

    fn process_mined_blocks(&mut self, chain: &mut BlockChain, blocks_done_recv: &Receiver<Block>, logger: &mut Logger<W>)
    {
        for block in blocks_done_recv.try_iter() 
        {
            self.blocks_being_mined -= 1;

            if block.block_id != chain.top_id() + 1
            {
                logger.log(LoggerLevel::Verbose, &format!("Mined block {} not at top {}", 
                    block.block_id, chain.top_id()));
                continue;
            }

            if block.validate(chain).is_err() 
            {
                logger.log(LoggerLevel::Verbose, "Mined block not valid");
                continue;
            }
            
            logger.log(LoggerLevel::Info, &format!("Accepted our block {}!!", block.block_id));
            for command in &mut self.commands {
                command.on_accepted_block(&block);
            }

            chain.add(block.clone(), logger);
            NetworkConnection::broadcast(&mut self.connection, 
                None, Packet::NewBlock(block));
        }
    }

    fn process_new_blocks_to_mine(&mut self, chain: &mut BlockChain, wallet: &PrivateWallet, 
        blocks_to_mine: &Sender<Block>, blocks_done: &Receiver<Block>, logger: &mut Logger<W>)
    {
        let next_block_needed = chain.next_block_needed();
        if next_block_needed != chain.top_id() + 1
        {
            NetworkConnection::request_block(&mut self.connection, 
                next_block_needed);
            return;
        }

        if self.blocks_being_mined == 0
        {
            let mut block = Block::new(chain, wallet).expect("Can make block");
            for command in &mut self.commands {
                command.on_create_block(&mut block);
            }

            self.blocks_being_mined += 1;
            blocks_to_mine.send(block).expect("Worked");
        }
        else
        {
            self.process_mined_blocks(chain, blocks_done, logger);
        }
    }

    pub fn run(&mut self, chain: &mut BlockChain, wallet: &PrivateWallet, logger: &mut Logger<W>)
    {
        NetworkConnection::run(self.connection.clone());

        if chain.top_id() != 0
        {
            NetworkConnection::broadcast(&mut self.connection, 
                None, Packet::NewBlock(chain.top().unwrap()));
        }
    
        let (blocks_to_mine_send, blocks_to_mine_recv) = channel::<Block>();
        let (blocks_done_send, blocks_done_recv) = channel::<Block>();
        std::thread::spawn(move || {
            Self::miner_thread(blocks_to_mine_recv, blocks_done_send);
        });

        loop
        {
            self.process_packets(chain, logger);
            self.process_mined_blocks(chain, &blocks_done_recv, logger);
            self.process_new_blocks_to_mine(chain, wallet, &blocks_to_mine_send, &blocks_done_recv, logger);

            std::thread::sleep(Duration::from_millis(100));
        }
    }

}
