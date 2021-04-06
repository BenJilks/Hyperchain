pub mod network;
mod broadcast;
mod command;
use crate::miner;
use crate::block::{Block, BlockChain};
use crate::wallet::PrivateWallet;
use crate::error::Error;
use network::{NetworkConnection, Packet};
use command::{Command, TransactionCommand, BalanceCommand};

use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Mutex, Arc};
use std::path::PathBuf;
use std::time::{Instant, Duration};
use rustyline::Editor;
use rustyline::config::Configurer;

pub struct Node
{
    connection: Arc<Mutex<NetworkConnection>>,
    top: u64,
    blocks_being_mined: i32,
    at_top: bool,
    start_prune_timer: Instant,

    commands: Vec<Box<dyn Command>>,
}

impl Node
{

    pub fn new(port: i32, known_nodes: PathBuf) -> Self
    {
        let connection = NetworkConnection::new(port, known_nodes).unwrap();
        Self
        {
            connection: Arc::from(Mutex::from(connection)),
            top: 0,
            blocks_being_mined: 0,
            at_top: false,
            start_prune_timer: Instant::now(),
            commands: vec![
                Box::from(TransactionCommand::default()),
                Box::from(BalanceCommand::default()),
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

    fn process_received_block(&mut self, chain: &mut BlockChain, block: Block)
    {
        //println!("Got block {}", block.block_id);
        match chain.add(&block)
        {
            Ok(_) => 
            {
                for command in &mut self.commands {
                    command.on_accepted_block(&block);
                }
                self.top = chain.top_id();
            },
            Err(Error::NoValidBranches) => self.top -= 1,
            Err(Error::DuplicateBlock) => {},
            Err(_) => panic!(),
        }
        NetworkConnection::set_top(&mut self.connection, self.top);
    }

    fn process_packets(&mut self, chain: &mut BlockChain)
    {
        for packet in NetworkConnection::process_packets(&mut self.connection)
        {
            match packet
            {
                Packet::NewBlock(block) => self.process_received_block(chain, block),
                _ =>
                {
                    for command in &mut self.commands {
                        command.on_packet(packet.clone(), &mut self.connection, chain);
                    }
                },
            }
        }
    }

    fn process_mined_blocks(&mut self, chain: &mut BlockChain, blocks_done_recv: &Receiver<Block>)
    {
        for block in blocks_done_recv.try_iter() 
        {
            self.blocks_being_mined -= 1;
            if block.block_id != chain.top_id() + 1 
            {
                //println!("Mined block {} not at top {}", block.block_id, chain.top_id());
                continue;
            }
            if block.validate(chain.longest_branch()).is_err() 
            {
                //println!("Mined block not valid on longest branch");
                continue;
            }
            if chain.add(&block).is_err() {
                continue;
            }
            
            //println!("Accepted our block {}!!", block.block_id);
            for command in &mut self.commands {
                command.on_accepted_block(&block);
            }
            self.top = chain.top_id();
            NetworkConnection::set_top(&mut self.connection, self.top);
        }
    }

    fn process_nodes(&mut self, chain: &mut BlockChain)
    {
        self.at_top = true;
        for (address, node) in NetworkConnection::nodes(&mut self.connection) 
        {
            if node.top < self.top 
            {
                let block = chain.longest_branch().block(node.top + 1);
                if block.is_some()
                {
                    NetworkConnection::broadcast(&mut self.connection, 
                        Some( address ), Packet::NewBlock(block.unwrap()));
                }
            }

            if node.top > self.top {
                self.at_top = false;
            }
        }
    }

    fn process_new_blocks_to_mine(&mut self, chain: &mut BlockChain, wallet: &PrivateWallet, blocks_to_mine_send: &Sender<Block>)
    {
        if self.at_top && self.blocks_being_mined == 0
        {
            let mut block = Block::new(chain.longest_branch(), wallet).expect("Can make block");
            for command in &mut self.commands {
                command.on_create_block(&mut block);
            }

            self.blocks_being_mined += 1;
            blocks_to_mine_send.send(block).expect("Worked");
        }
    }

    fn prune_branches(&mut self, chain: &mut BlockChain)
    {
        if self.start_prune_timer.elapsed().as_secs() >= 1 
        {
            chain.prune_branches();
            self.start_prune_timer = Instant::now();
        }
    }

    fn process_commands(&mut self, lines_recv: &Receiver<Vec<String>>, chain: &mut BlockChain)
    {
        for line in lines_recv.try_iter() 
        {
            if line.len() == 0 {
                continue;
            }

            for command in &mut self.commands 
            {
                if command.name() == line[0] {
                    command.invoke(&line[1..], &mut self.connection, chain);
                }
            }
        }
    }

    fn command_line_thread(send: Sender<Vec<String>>)
    {
        let mut line_editor = Editor::<()>::new();
        line_editor.set_auto_add_history(true);

        loop
        {
            match line_editor.readline(">> ")
            {
                Ok(line) => 
                {
                    send.send(line.split(' ')
                        .map(|x| x.to_owned())
                        .collect::<Vec<String>>()).unwrap();
                },

                Err(err) => 
                {
                    println!("Error: {:?}", err);
                    break;
                },
            }
        }
    }

    pub fn run(&mut self, chain: &mut BlockChain, wallet: &PrivateWallet)
    {
        self.top = chain.top_id();
        NetworkConnection::run(self.connection.clone());
        NetworkConnection::set_top(&mut self.connection, self.top);

        let (blocks_to_mine_send, blocks_to_mine_recv) = channel::<Block>();
        let (blocks_done_send, blocks_done_recv) = channel::<Block>();
        std::thread::spawn(move || {
            Self::miner_thread(blocks_to_mine_recv, blocks_done_send);
        });

        let (lines_send, lines_recv) = channel::<Vec<String>>();
        std::thread::spawn(move || {
            Self::command_line_thread(lines_send);
        });

        std::thread::sleep(Duration::from_secs(5));

        loop
        {
            self.process_packets(chain);
            self.process_mined_blocks(chain, &blocks_done_recv);
            self.process_nodes(chain);
            self.process_new_blocks_to_mine(chain, wallet, &blocks_to_mine_send);
            self.prune_branches(chain);
            self.process_commands(&lines_recv, chain);

            std::thread::sleep(Duration::from_millis(100));
        }
    }

}
