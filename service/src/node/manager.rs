use super::packet_handler::{Packet, Message};
use super::connection::Connection;

use tcp_channel::ChannelSend;
use std::net::{TcpStream, SocketAddr};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, Arc};
use std::collections::{HashSet, HashMap};
use std::time::Duration;

pub struct ConnectionManager
{
    pub(crate) port: u16,
    message_sender: Sender<Message>,
    known_nodes: HashSet<String>,
    pub(crate) open_connections: HashSet<String>,
    connections: HashMap<String, Connection>,
}

impl ConnectionManager
{

    pub fn new(port: u16, message_sender: Sender<Message>) -> Arc<Mutex<Self>>
    {
        Arc::from(Mutex::from(Self
        {
            port,
            message_sender,
            known_nodes: HashSet::new(),
            open_connections: HashSet::new(),
            connections: HashMap::new(),
        }))
    }

    pub(crate) fn add_client(&mut self, address: String, stream: TcpStream)
    {
        info!("[{}] Connected to {}", self.port, address);

        match Connection::new(self.port, &address, stream, self.message_sender.clone())
        {
            Ok(connection) => {
                self.connections.insert(address, connection);
            },

            _ => {},
        };
    }

    pub(crate) fn confirm_connection(&mut self, address: &str, public_address: String)
    {
        let connection = &mut self.connections.get_mut(address).unwrap();
        connection.public_address = Some( public_address );
    }

    pub fn register_node(&mut self, address: &str, from: Option<&str>)
    {
        if self.known_nodes.insert(address.to_owned())
        {
            debug!("[{}] Regestered new node {}", self.port, address);
            if from.is_some() {
                self.send_to(Packet::KnownNode(address.to_owned()), |addr| addr != from.unwrap());
            }
        }
    }

    fn connect(&mut self, address: &str)
    {
        // TODO: Test we're not connecting to our self properly
        if address == &format!("127.0.0.1:{}", self.port) {
            return;
        }

        // Make sure we're not already connected
        if self.open_connections.contains(address)
        {
            warn!("[{}] Already to connected to {}", self.port, address);
            return;
        }

        let socket_address = address.parse::<SocketAddr>();
        if socket_address.is_err() {
            return;
        }

        match TcpStream::connect_timeout(
            &socket_address.unwrap(), Duration::from_millis(100))
        {
            Ok(stream) =>
                self.add_client(address.to_owned(), stream),

            Err(_) =>
                debug!("[{}] Unable to connect to {}", self.port, address),
        }
    }

    pub fn connect_to_known_nodes(&mut self)
    {
        // TODO: Limit the number of connections we make

        for address in self.known_nodes.clone() 
        {
            if !self.open_connections.contains(&address) {
                self.connect(&address);
            }
        }
    }

    pub fn send(&mut self, packet: Packet)
    {
        self.send_to(packet, |_| true);
    }

    pub fn send_to<F>(&mut self, packet: Packet, predicate: F)
        where F: Fn(&str) -> bool
    {
        let mut disconnected = Vec::<String>::new();
        for (address, connection) in &mut self.connections
        {
            if connection.public_address.is_none() {
                continue;
            }

            debug!("[{}] Sending {:?} to {}", self.port, packet, address);
            if predicate(address)
            {
                if connection.sender.send(&packet).is_err() 
                    || connection.sender.flush().is_err()
                {
                    disconnected.push(address.clone());
                }
            }
        }

        // Remove any 
        for address in disconnected {
            self.disconnect_from(&address);
        }
    }

    pub fn disconnect_from(&mut self, address: &str)
    {
        match self.connections.remove(address)
        {
            Some(connection) => 
                match &connection.public_address
                {
                    Some(address) => self.open_connections.remove(address),
                    None => false,
                },

            None => false,
        };
    }

}

impl Drop for ConnectionManager
{

    fn drop(&mut self)
    {
        info!("[{}] Shutting down {} connection(s)", self.port, self.connections.len());
        self.connections.clear();
    }

}
