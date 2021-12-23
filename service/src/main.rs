extern crate libhyperchain;
extern crate clap;
extern crate rand;
extern crate pretty_env_logger;
extern crate serde_json;

#[macro_use]
extern crate slice_as_array;

#[macro_use]
extern crate log;

mod node;
mod network;
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
use blocks::{blocks, top_block};
use crate::network::NetworkConnection;
use crate::node::Node;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::server;
use libhyperchain::service::command::{Command, Response};
use clap::{App, Arg};
use std::error::Error;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>>
{
    pretty_env_logger::init();

    let matches = App::new("Hyperchain Cli")
        .version("0.2.0")
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
    let port = matches.value_of("port").unwrap_or("8001").parse::<u16>().unwrap();
    let disable_local_server = matches.is_present("local-server");

    // Create and open node
    let data_directory = PathBuf::from("hyperchain");
    let node = Node::new(port, &data_directory)?;
    let packet_handler = NodePacketHandler::new(node);

    let miner_thread;
    {
        // Register a common node to connect to
        let mut network_connection = NetworkConnection::open(port, &data_directory, packet_handler)?;
        network_connection.manager().register_node("192.168.0.52:8001");

        // Start miner thread
        miner_thread = start_miner_thread(network_connection.clone());
        if disable_local_server
        {
            miner_thread.join().unwrap();
            return Ok(());
        }

        // Start local server
        let mut connection = network_connection.clone();
        server::start(move |command|
        {
            match command
            {
                Command::Exit => 
                    Response::Exit,

                Command::Balance(address) => 
                    balance(&mut connection, address),

                Command::Send(inputs, outputs, fee) =>
                    send(&mut connection, inputs, outputs, fee),

                Command::UpdatePage(from, name, data) =>
                    update_page(&mut connection, from, name, data),

                Command::TransactionInfo(id) =>
                    transaction_info(&mut connection, id),
                
                Command::TransactionHistory(address) =>
                    transaction_history(&mut connection, address),
                
                Command::PageUpdates(address) =>
                    page_updates(&mut connection, address),

                Command::PageData(transaction_id) =>
                    page_data(&mut connection, transaction_id),
                
                Command::Blocks(from, to) =>
                    blocks(&mut connection, from, to),

                Command::TopBlock =>
                    top_block(&mut connection),
            }
        })?;
    }

    miner_thread.join().unwrap();
    Ok(())
}

