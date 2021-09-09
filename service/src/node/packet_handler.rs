use super::network::NetworkConnection;
use super::manager::ConnectionManager;

use libhyperchain::logger::LoggerLevel;
use libhyperchain::block::Block;
use libhyperchain::transaction::Transaction;
use libhyperchain::transaction::transfer::Transfer;
use libhyperchain::transaction::page::Page;
use libhyperchain::data_store::DataUnit;
use libhyperchain::config::Hash;

use serde::{Serialize, Deserialize};
use std::io::Write;
use std::thread::JoinHandle;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Mutex, Arc};
use std::collections::HashMap;
use std::time::Duration;
use std::error::Error;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet
{
    KnownNode(String),
    OnConnected(u16),
    Block(Block, HashMap<Hash, DataUnit>),
    BlockRequest(u64),
    Transfer(Transaction<Transfer>),
    Page(Transaction<Page>, DataUnit),
    Ping,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message
{
    Packet(String, Packet),
    Shutdown,
}

pub trait PacketHandler<W>
    where W: Write + Clone + Sync + Send + 'static
{
    fn on_packet(&mut self, from: &str, packet: Packet, connection_manager: &mut ConnectionManager<W>) 
        -> Result<(), Box<dyn Error>>;
}

fn handle_message_packet<P, W>(from: String, packet: Packet, 
                               network_connection: &mut NetworkConnection<P, W>)
    where P: PacketHandler<W> + Sync + Send + 'static,
          W: Write + Clone + Sync + Send + 'static
{
    let port = network_connection.port;
    let handle_packet = |network_connection: &mut NetworkConnection<P, W>, packet, manager_lock|
    {
        match network_connection.handler().on_packet(&from, packet, manager_lock)
        {
            Ok(_) => {},
            Err(err) =>
            {
                network_connection.logger.log(
                    LoggerLevel::Error, &format!("Error handling packet: {}", err))
            },
        }
    };

    match &packet
    {
        // NOTE: We don't send KnownNode packets to the handler
        Packet::KnownNode(address) =>
            network_connection.manager().register_node(&address, Some( &from )),

        Packet::OnConnected(node_port) =>
        {
            let ip = from.split(':').nth(0).unwrap();
            let node_address = format!("{}:{}", ip, node_port);
            if !network_connection.manager().open_connections.insert(node_address.clone())
            {
                network_connection.logger.log(LoggerLevel::Verbose, 
                    &format!("[{}] Remove duplicate connection {}", port, node_address));
                network_connection.manager().disconnect_from(&from);
            }
            else
            {
                network_connection.manager().confirm_connection(&from, node_address.clone());
                network_connection.manager().register_node(&node_address, Some( &from ));

                let manager = network_connection.connection_manager.clone().unwrap();
                let mut manager_lock = manager.lock().unwrap();
                handle_packet(network_connection, packet, &mut manager_lock);
            }
        },

        _ => 
        {
            let manager = network_connection.connection_manager.clone().unwrap();
            let mut manager_lock = manager.lock().unwrap();
            handle_packet(network_connection, packet, &mut manager_lock);
        },
    }
}

pub fn start_message_handler<P, W>(network_connection: Arc<Mutex<NetworkConnection<P, W>>>, 
                                   message_reciver: Receiver<Message>) -> JoinHandle<()>
    where P: PacketHandler<W> + Sync + Send + 'static,
          W: Write + Clone + Sync + Send + 'static
{
    std::thread::spawn(move || loop
    {
        match message_reciver.recv_timeout(Duration::from_millis(100))
        {
            Ok(Message::Packet(from, packet)) =>
            {
                let mut network_connection_lock = network_connection.lock().unwrap();
                let port = network_connection_lock.port;
                network_connection_lock.logger.log(LoggerLevel::Verbose, 
                    &format!("[{}] Got packet {:?}", port, packet));
                network_connection_lock.logger.log(LoggerLevel::Verbose, &format!("[{}] Got packet", port));

                handle_message_packet(from, packet, &mut network_connection_lock);
                network_connection_lock.logger.log(LoggerLevel::Verbose, &format!("[{}] Handled packet", port));
            },

            Ok(Message::Shutdown) =>
            {
                let mut network_connection_lock = network_connection.lock().unwrap();
                let port = network_connection_lock.port;
                let logger = &mut network_connection_lock.logger;
                logger.log(LoggerLevel::Info, 
                    &format!("[{}] Shut down message handler", port));
                break;
            },

            Err(RecvTimeoutError::Timeout) =>
            {
                let mut network_connection_lock = network_connection.lock().unwrap();
                let connection_manager = &mut network_connection_lock.manager();
                connection_manager.connect_to_known_nodes();
            },

            // TODO: Handle this
            Err(err) =>
            {
                let mut network_connection_lock = network_connection.lock().unwrap();
                let logger = &mut network_connection_lock.logger;
                logger.log(LoggerLevel::Error, 
                    &format!("message_reciver.recv(): {}", err));
                panic!()
            },
        }
    })
}

