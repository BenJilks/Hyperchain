use super::packet::{Packet, PacketHandler};
use super::packet::{Message, MessageSender};
use super::client::client_handler_thread;

use tcp_channel::ChannelSend;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::collections::HashSet;
use std::error::Error;

struct ClientSender
{
    address: String,
    sender: MessageSender,
}

struct ClientReceiver
{
    stream: TcpStream,
}

struct ConnectionData
{
    client_senders: Vec<ClientSender>,
    client_receivers: Vec<ClientReceiver>,

    known_nodes: HashSet<String>,
    connected_nodes: HashSet<String>,
}

impl Default for ConnectionData
{

    fn default() -> Self
    {
        Self
        {
            client_senders: Vec::new(),
            client_receivers: Vec::new(),

            known_nodes: HashSet::new(),
            connected_nodes: HashSet::new(),
        }
    }

}

#[derive(Clone)]
pub struct ClientManager
{
    port: u16,
    shutdown_signal: Arc<Mutex<bool>>,
    data: Arc<Mutex<ConnectionData>>,
}

impl ClientManager
{

    pub fn new(port: u16, shutdown_signal: Arc<Mutex<bool>>) -> Self
    {
        ClientManager
        {
            port,
            shutdown_signal,
            data: Default::default(),
        }
    }

    pub fn port(&self) -> u16
    {
        self.port
    }

    pub fn should_shutdown(&self) -> bool
    {
        *self.shutdown_signal.lock().unwrap()
    }

    pub fn register_node(&mut self, address: &str) -> bool
    {
        if address == format!("127.0.0.1:{}", self.port) {
            return false;
        }
        
        let mut data = self.data.lock().unwrap();
        if !data.known_nodes.insert(address.to_owned()) {
            return false;
        }

        info!("[{}] Discovered new node {}", self.port, address);
        true
    }

    pub fn get_not_connected_nodes(&self) -> Vec<String>
    {
        let data = self.data.lock().unwrap();
        data.known_nodes
            .iter()
            .filter(|x| !data.connected_nodes.contains(*x))
            .map(|x| x.to_owned())
            .collect()
    }

    pub fn new_client<H>(&mut self, packet_handler: H, stream: TcpStream, ip: String)
        -> Result<(), Box<dyn Error>>
        where H: PacketHandler + Clone + Sync + Send + 'static
    {
        client_handler_thread(
            packet_handler,
            self.clone(),
            stream.try_clone().unwrap(), ip)?;

        let mut data = self.data.lock().unwrap();
        data.client_receivers.push(ClientReceiver
        {
            stream,
        });

        Ok(())
    }

    pub fn register_client_sender(&mut self, address: String, 
                                  mut sender: MessageSender)
        -> Result<(), Box<dyn Error>>
    {
        let mut data = self.data.lock().unwrap();
        data.connected_nodes.insert(address.clone());
        data.known_nodes.insert(address.clone());

        // Send all our known nodes over
        for node in &data.known_nodes
        {
            if node != &address {
                sender.send(&Message::KnownNode(node.clone()))?;
            }
        }
        sender.flush()?;

        data.client_senders.push(ClientSender
        {
            address,
            sender,
        });

        Ok(())
    }

    pub fn register_disconnect(&mut self, address: &str)
    {
        info!("[{}] Client {} disconnected", self.port, address);

        let mut data = self.data.lock().unwrap();
        data.connected_nodes.remove(address);
        data.client_senders.retain(|x| x.address != address);
    }

    pub fn send_message_to<F>(&mut self, message: Message, mut predicate: F)
        -> Result<(), Box<dyn Error>>
        where F: FnMut(&str) -> bool
    {
        let mut disconnected_clients = Vec::new();
        {
            let mut data = self.data.lock().unwrap();
            for connection in &mut data.client_senders
            {
                if predicate(&connection.address)
                {
                    let send_result = connection.sender.send(&message);
                    let flush_result = connection.sender.flush();
                    if send_result.is_err() || flush_result.is_err() {
                        disconnected_clients.push(connection.address.clone());
                    }
                }
            }
        }

        for address in disconnected_clients {
            self.register_disconnect(&address);
        }
        Ok(())
    }

    pub fn send_to<F>(&mut self, packet: Packet, predicate: F)
        -> Result<(), Box<dyn Error>>
        where F: FnMut(&str) -> bool
    {
        self.send_message_to(Message::Packet(packet), predicate)
    }

    pub fn send(&mut self, packet: Packet)
        -> Result<(), Box<dyn Error>>
    {
        self.send_to(packet, |_| true)
    }

    pub fn shutdown(&mut self)
    {
        self.data.lock().unwrap().shutdown(self.port);
    }

}

impl ConnectionData
{

    fn shutdown(&mut self, port: u16)
    {
        info!("[{}] Closing {} open client(s)", port, self.client_senders.len());
        for _ in 0..self.client_receivers.len()
        {
            let receiver = self.client_receivers.remove(0);
            let _ = receiver.stream.shutdown(std::net::Shutdown::Both);
        }
    }

}

