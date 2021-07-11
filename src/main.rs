#[macro_use] 
extern crate slice_as_array;

#[cfg(feature = "serde_derive")] 
#[macro_use]
extern crate serde;

#[macro_use]
extern crate serde_big_array;

extern crate sha2;
extern crate bincode;
extern crate rsa;
extern crate rand;
extern crate num_traits;
extern crate bidiff;
extern crate bipatch;
extern crate base_62;
extern crate serde_json;

mod block;
mod miner;
mod wallet;
mod error;
mod logger;
mod node;
use logger::{Logger, LoggerLevel, StdLoggerOutput};
use block::{Block, BlockChain};
use node::Node;
use node::network::{NetworkConnection, Packet};
use wallet::PrivateWallet;

use std::sync::{Arc, Mutex};
use std::path::PathBuf;

// TODO: This will all be replaced with a daemon/ipc system
fn main()
{
    println!("Hello, Blockchains!!");

    // Crate logger and read port from command line
    let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Info);
    let port = std::env::args().nth(1).unwrap().parse::<u16>().unwrap();

    // Create chain a wallet
    let chain = Arc::from(Mutex::from(BlockChain::new(&mut logger)));
    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

    // Create and open node
    let node = Node::new(port, chain.clone(), logger.clone());
    let mut network_connection = NetworkConnection::new(port, node, logger.clone());

    // Register a common node to connect to
    network_connection.sender().register_node("127.0.0.1:8001", None);
    
    loop
    {
        // Mine the next block
        let mut block = Block::new(chain.lock().unwrap().current_branch(), &wallet).unwrap();
        block = miner::mine_block(block);

        {
            // Add it to the chain if it's still the top
            // TODO: Cancel the mining if we know this already
            let mut chain_lock = chain.lock().unwrap();
            let branch = chain_lock.current_branch().unwrap();
            if branch.top().block_id < block.block_id 
            {
                chain_lock.add(&block, &mut logger);
                network_connection.sender().send(Packet::Block(block));
            }
        }
    }
}

