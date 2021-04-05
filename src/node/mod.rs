mod network;
mod broadcast;
use crate::miner;
use crate::block::{Block, BlockChain};
use crate::wallet::PrivateWallet;
use crate::error::Error;
use network::{NetworkConnection, Packet};

use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Mutex, Arc};
use std::path::PathBuf;
use std::time::Instant;

pub struct Node
{
    connection: Arc<Mutex<NetworkConnection>>,
}

impl Node
{

    pub fn new(port: i32, known_nodes: PathBuf) -> Self
    {
        let connection = NetworkConnection::new(port, known_nodes).unwrap();
        Self
        {
            connection: Arc::from(Mutex::from(connection)),
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

    pub fn run(&mut self, chain: &mut BlockChain, wallet: &PrivateWallet)
    {
        let mut top = chain.top_id();
        NetworkConnection::run(self.connection.clone());
        NetworkConnection::set_top(&mut self.connection, top);

        let (blocks_to_mine_send, blocks_to_mine_recv) = channel::<Block>();
        let (blocks_done_send, blocks_done_recv) = channel::<Block>();
        let mut blocks_being_mined = 0;
        std::thread::spawn(move || {
            Self::miner_thread(blocks_to_mine_recv, blocks_done_send);
        });

        let mut start_prune_timer = Instant::now();
        loop
        {
            for block in NetworkConnection::process_new_blocks(&mut self.connection) 
            {
                println!("Got block {}", block.block_id);
                match chain.add(&block)
                {
                    Ok(_) => top = chain.top_id(),
                    Err(Error::NoValidBranches) => top -= 1,
                    Err(Error::DuplicateBlock) => {},
                    Err(_) => panic!(),
                }
                NetworkConnection::set_top(&mut self.connection, top);
            }
            
            for block in blocks_done_recv.try_iter() 
            {
                blocks_being_mined -= 1;
                if block.block_id != chain.top_id() + 1 
                {
                    println!("Mined block {} not at top {}", block.block_id, chain.top_id());
                    continue;
                }
                if block.validate(chain.longest_branch()).is_err() 
                {
                    println!("Mined block not valid on longest branch");
                    continue;
                }
                if chain.add(&block).is_err() {
                    continue;
                }

                println!("Accepted our block {}!!", block.block_id);
                top = chain.top_id();
                NetworkConnection::set_top(&mut self.connection, top);
            }

            let mut at_top = true;
            for (address, node) in NetworkConnection::nodes(&mut self.connection) 
            {
                if node.top < top 
                {
                    let block = chain.longest_branch().block(node.top + 1);
                    if block.is_some()
                    {
                        NetworkConnection::broadcast(&mut self.connection, 
                            Some( address ), Packet::NewBlock(block.unwrap()));
                    }
                }

                if node.top > top {
                    at_top = false;
                }
            }

            if at_top && blocks_being_mined == 0
            {
                let block = Block::new(chain.longest_branch(), wallet).expect("Can make block");
                blocks_being_mined += 1;
                blocks_to_mine_send.send(block).expect("Worked");
            }

            std::thread::sleep_ms(100);
            if start_prune_timer.elapsed().as_secs() >= 1 
            {
                chain.prune_branches();
                start_prune_timer = Instant::now();
            }
        }
    }

}
