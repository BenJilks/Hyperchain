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
mod chain;
mod transaction;
mod page;
mod miner;
mod wallet;
mod error;
mod logger;
mod node;
use logger::{Logger, LoggerLevel, StdLoggerOutput};
use chain::{BlockChain, BlockChainAddResult};
use block::Block;
use block::validate::BlockValidate;
use node::Node;
use node::network::{NetworkConnection, Packet};
use wallet::PrivateWallet;

use std::path::PathBuf;
use std::io::Write;
use std::sync::{Arc, Mutex};

fn mine_next_block<W>(network_connection: &Arc<Mutex<NetworkConnection<Node<W>, W>>>,
                      wallet: &PrivateWallet)
    where W: Write + Clone + Sync + Send + 'static,
{
    let mut block;
    {
        // Create the next block
        let mut network_connection_lock = network_connection.lock().unwrap();
        let chain = &network_connection_lock.handler().chain();
        block = Block::new(&chain, wallet).unwrap();
    }

    // Do the mining work
    block = miner::mine_block_unless_found(network_connection, block);
    if !block.is_pow_valid() {
        return;
    }

    // Add it to the chain if it's still the top
    let mut network_connection_lock = network_connection.lock().unwrap();
    let chain = &mut network_connection_lock.handler().chain();
    let top = chain.top();
    if top.is_none() || top.unwrap().block_id + 1 == block.block_id 
    {
        match chain.add(&block)
        {
            BlockChainAddResult::Ok =>
            {
                println!("Won block {}! With difficulty {}", 
                    block.block_id, 
                    block::target::difficulty(&block.target));

                network_connection_lock.manager().send(Packet::Block(block));
            },

            _ => {},
        }
    }
}

// TODO: This will all be replaced with a daemon/ipc system
fn main()
{
    println!("Hello, Blockchains!!");

    // Crate logger and read port from command line
    let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Info);
    let port = std::env::args().nth(1).unwrap().parse::<u16>().unwrap();

    // Create chain a wallet
    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

    // Create and open node
    let chain = BlockChain::new(&mut logger);
    let node = Node::new(port, chain, logger.clone());
    let network_connection = NetworkConnection::new(port, node, logger.clone());

    // Register a common node to connect to
    network_connection.lock().unwrap().manager().register_node("127.0.0.1:8001", None);
    
    loop {
        mine_next_block(&network_connection, &wallet);
    }
}

