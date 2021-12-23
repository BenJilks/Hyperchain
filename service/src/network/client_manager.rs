use super::packet::{Packet, PacketHandler};
use super::packet::{Message, MessageSender};
use super::client::client_handler_thread;

use serde_json;
use serde::{Serialize, Deserialize};
use tcp_channel::ChannelSend;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::collections::{HashSet, HashMap};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::path::PathBuf;
use std::fs::File;
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

#[derive(Serialize, Deserialize)]
struct NodeConnectionInformation
{
    ping_time_samples: Vec<Duration>,
}

impl NodeConnectionInformation
{

    pub fn add_sample(&mut self, sample: Duration)
    {
        self.ping_time_samples.push(sample);
        if self.ping_time_samples.len() > 10 {
            self.ping_time_samples.remove(0);
        }
    }

    pub fn average_ping_time(&self) -> Option<Duration>
    {
        let count = self.ping_time_samples.len() as u32;
        if count == 0 {
            return None;
        }

        let sum = self.ping_time_samples.iter().sum::<Duration>();
        Some(sum / count)
    }

}

impl Default for NodeConnectionInformation
{

    fn default() -> Self
    {
        Self
        {
            ping_time_samples: Vec::new(),
        }
    }

}

struct ConnectionData
{
    client_senders: Vec<ClientSender>,
    client_receivers: Vec<ClientReceiver>,

    data_directory: PathBuf,
    known_nodes: HashMap<String, NodeConnectionInformation>,
    connected_nodes: HashSet<String>,
}

impl ConnectionData
{

    pub fn new(data_directory: &PathBuf) -> Arc<Mutex<Self>>
    {
        let known_nodes = Self::existing_known_nodes(data_directory)
            .unwrap_or(HashMap::new());

        Arc::from(Mutex::from(Self
        {
            client_senders: Vec::new(),
            client_receivers: Vec::new(),

            data_directory: data_directory.clone(),
            known_nodes,
            connected_nodes: HashSet::new(),
        }))
    }

    fn existing_known_nodes(data_directory: &PathBuf) 
        -> Result<HashMap<String, NodeConnectionInformation>, Box<dyn Error>>
    {
        let known_nodes_path = data_directory.join("known_nodes.json");
        let known_nodes_file = File::open(&known_nodes_path)?;
        Ok(serde_json::from_reader(known_nodes_file)?)
    }

    pub fn flush_changes(&self) 
        -> Result<(), Box<dyn Error>>
    {
        let known_nodes_path = self.data_directory.join("known_nodes.json");
        let known_nodes_file = File::create(&known_nodes_path)?;
        serde_json::to_writer_pretty(known_nodes_file, &self.known_nodes)?;

        Ok(())
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

    pub fn new(port: u16, data_directory: &PathBuf, 
               shutdown_signal: Arc<Mutex<bool>>) 
        -> Self
    {
        ClientManager
        {
            port,
            shutdown_signal,
            data: ConnectionData::new(data_directory),
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
        if data.known_nodes.contains_key(address) {
            return false;
        }

        info!("[{}] Discovered new node {}", self.port, address);
        data.known_nodes.insert(address.to_owned(), Default::default());
        data.flush_changes().expect("Can flush changes");
        true
    }

    pub fn get_not_connected_nodes(&self) -> Vec<String>
    {
        let data = self.data.lock().unwrap();
        data.known_nodes
            .iter()
            .filter(|(x, _)| !data.connected_nodes.contains(*x))
            .map(|(x, _)| x.to_owned())
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

        if !data.known_nodes.contains_key(&address) 
        {
            data.known_nodes.insert(address.clone(), Default::default());
            data.flush_changes()?;
        }

        // Send all our known nodes over
        for (node, _) in &data.known_nodes
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

    pub fn report_ping_time(&mut self, from: &str, time_sent_nanos: u128)
    {
        let mut data = self.data.lock().unwrap();
        if !data.known_nodes.contains_key(from) {
            return;
        }

        let current_time_nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_nanos();

        let node_info = data.known_nodes.get_mut(from).unwrap();
        let time_taken = Duration::from_nanos((current_time_nanos - time_sent_nanos) as u64);
        node_info.add_sample(time_taken);
        data.flush_changes().expect("Can flush changes");
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
                if !predicate(&connection.address) {
                    continue;
                }

                let send_result = connection.sender.send(&message);
                let flush_result = connection.sender.flush();
                if send_result.is_err() || flush_result.is_err() {
                    disconnected_clients.push(connection.address.clone());
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

