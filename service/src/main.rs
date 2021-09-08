extern crate libhyperchain;
extern crate clap;

#[macro_use]
extern crate slice_as_array;

mod node;
mod block_builder;
mod miner;
mod send;
mod update_page;
mod balance;
mod transaction_info;
mod transaction_history;
mod page;
mod blocks;
use miner::start_miner_thread;
use send::send;
use update_page::update_page;
use balance::balance;
use transaction_history::transaction_history;
use transaction_info::transaction_info;
use page::page_updates;
use page::page_data;
use blocks::blocks;
use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::logger::{Logger, LoggerLevel, StdLoggerOutput};
use libhyperchain::service::server;
use libhyperchain::service::command::{Command, Response};
use clap::{App, Arg};
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>>
{
    let matches = App::new("Hyperchain Cli")
        .version("0.1.0")
        .author("Ben Jilks <benjyjilks@gmail.com>")
        .about("Hyperchain node service process")
        .arg(Arg::with_name("port")
            .short("p")
            .long("port")
            .takes_value(true)
            .required(false)
            .help("Port of main node connection"))
        .arg(Arg::with_name("local-server")
            .short("l")
            .long("local-server")
            .takes_value(false)
            .required(false)
            .help("Disable running local server"))
        .get_matches();

    // Crate logger and read port from command line
    let logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Info);
    let port = matches.value_of("port").unwrap_or("8001").parse::<u16>().unwrap();
    let disable_local_server = matches.is_present("local-server");

    // Create and open node
    let node = Node::new(port, PathBuf::from("hyperchain"), logger.clone())?;

    let miner_thread;
    {
        // Register a common node to connect to
        let network_connection = NetworkConnection::new(port, node, logger.clone());
        network_connection.lock().unwrap().manager().register_node("192.168.0.27:8001", None);

        // Start miner thread
        miner_thread = start_miner_thread(network_connection.clone(), logger.clone());
        if disable_local_server
        {
            miner_thread.join().unwrap();
            return Ok(());
        }

        // Start local server
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

                Command::UpdatePage(from, name, data) =>
                    update_page(&mut connection.lock().unwrap(), from, name, data),

                Command::TransactionInfo(id) =>
                    transaction_info(&mut connection.lock().unwrap(), id),
                
                Command::TransactionHistory(address) =>
                    transaction_history(&mut connection.lock().unwrap(), address),
                
                Command::PageUpdates(address) =>
                    page_updates(&mut connection.lock().unwrap(), address),

                Command::PageData(transaction_id) =>
                    page_data(&mut connection.lock().unwrap(), transaction_id),
                
                Command::Blocks(from, to) =>
                    blocks(&mut connection.lock().unwrap(), from, to),
            }
        })?;

        NetworkConnection::shutdown(&network_connection);
    }

    miner_thread.join().unwrap();
    Ok(())
}
