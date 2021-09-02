extern crate libhyperchain;

#[macro_use]
extern crate slice_as_array;

mod node;
mod block_builder;
mod miner;
mod send;
mod balance;
mod transaction_info;
mod transaction_history;
mod blocks;
use miner::start_miner_thread;
use send::send;
use balance::balance;
use transaction_history::transaction_history;
use transaction_info::transaction_info;
use blocks::blocks;
use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::chain::BlockChain;
use libhyperchain::logger::{Logger, LoggerLevel, StdLoggerOutput};
use libhyperchain::service::server;
use libhyperchain::service::command::{Command, Response};
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>>
{
    // Crate logger and read port from command line
    let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Info);
    let port = std::env::args().nth(1).unwrap().parse::<u16>()?;

    // Create and open node
    let chain = BlockChain::open(&PathBuf::from("blockchain"), &mut logger)?;
    let node = Node::new(port, chain, logger.clone());

    let miner_thread;
    {
        let network_connection = NetworkConnection::new(port, node, logger.clone());

        // Register a common node to connect to
        network_connection.lock().unwrap().manager().register_node("127.0.0.1:8001", None);
        
        miner_thread = start_miner_thread(network_connection.clone(), logger.clone());

        let connection = network_connection.clone();
        server::start(move |command|
        {
            match command
            {
                Command::Exit => 
                    Response::Exit,

                Command::Balance(address) => 
                    balance(&mut connection.lock().unwrap(), address),

                Command::Send(from, to, amount, fee) =>
                    send(&mut connection.lock().unwrap(), from, to, amount, fee),

                Command::TransactionInfo(id) =>
                    transaction_info(&mut connection.lock().unwrap(), id),
                
                Command::TransactionHistory(address) =>
                    transaction_history(&mut connection.lock().unwrap(), address),
                
                Command::Blocks(from, to) =>
                    blocks(&mut connection.lock().unwrap(), from, to),
            }
        })?;

        NetworkConnection::shutdown(&network_connection);
    }

    miner_thread.join().unwrap();
    Ok(())
}

