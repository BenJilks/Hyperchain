use crate::block::{Block, Transaction};
use crate::wallet::{Wallet, PublicWallet};
use super::broadcast::Broadcaster;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::fs::File;
use std::sync::mpsc::{self, channel, Sender, Receiver};
use std::sync::{Mutex, Arc};
use std::time::Duration;
use std::collections::{HashSet, HashMap};
use tcp_channel::{ReceiverBuilder, ChannelRecv};
use tcp_channel::{SenderBuilder, ChannelSend};
use tcp_channel::LittleEndian;
use serde::{Serialize, Deserialize};

pub const THIS_NODE_ID: &str = "<<THIS NODE>>";

type TCPReceiver = tcp_channel::Receiver<Packet, LittleEndian>;
type TCPSender = tcp_channel::Sender<Packet, LittleEndian>;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OtherNode
{
    pub ip: String,
    pub port: i32,
    pub top: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Packet
{
    Hello(OtherNode),
    KnownNode(String),
    NewBlock(Block),

    TransactionRequest(Transaction),
    TransactionRequestAccepted(Transaction),
    TransactionRequestRejected(Transaction),
    Ping,
}

pub struct NetworkConnection
{
    port: i32,
    known_nodes: HashSet<String>,
    know_nodes_path: PathBuf,
    open_connections: Arc<Mutex<HashSet<String>>>,
    broadcaster: Broadcaster<(Option<String>, Packet)>,

    other_nodes: HashMap<String, OtherNode>,
    packet_queue: Vec<Packet>,
    my_top: u64,
}

impl NetworkConnection
{

    pub fn new(port: i32, know_nodes_path: PathBuf) -> std::io::Result<Self>
    {
        let known_nodes: HashSet<String> = 
            if know_nodes_path.exists() {
                serde_json::from_reader(File::open(&know_nodes_path)?).unwrap()
            } else {
                HashSet::new()
            };

        Ok(Self
        {
            port,
            known_nodes,
            know_nodes_path,
            open_connections: Arc::from(Mutex::from(HashSet::new())),
            broadcaster: Broadcaster::new(),

            other_nodes: HashMap::new(),
            packet_queue: Vec::new(),
            my_top: 0,
        })
    }

    fn server(mut send: TCPSender, address: String, recv_broadcast: Receiver<(Option<String>, Packet)>, open_connections: Arc<Mutex<HashSet<String>>>)
    {
        for broadcast in recv_broadcast 
        {
            if broadcast.0.is_some() 
            {
                if broadcast.0.unwrap() != address {
                    continue;
                }
            }

            if send.send(&broadcast.1).is_err() {
                break;
            }
            if send.flush().is_err() {
                break;
            }
        }

        open_connections.lock().unwrap().remove(&address);
    }

    pub fn update_known_nodes(&mut self, address: &str)
    {
        if self.known_nodes.contains(address) {
            return;
        }

        self.known_nodes.insert(address.to_owned());
        let mut file = File::create(&self.know_nodes_path).unwrap();
        file.write(&serde_json::to_vec(&self.known_nodes).unwrap()).unwrap();
    }

    fn connect(&mut self, address: &str) -> std::io::Result<()>
    {
        self.update_known_nodes(address);

        // If this connection is already open, ignore it
        if self.open_connections.lock().unwrap().contains(address) {
            return Ok(());
        }
        self.open_connections.lock().unwrap().insert(address.to_owned());

        let port = self.port;
        let address_owned = address.to_owned();
        let open_connections = self.open_connections.clone();
        let recv_broadcast = self.broadcaster.make_receiver();
        let known_nodes = self.known_nodes.clone();
        std::thread::spawn(move || 
        {
            let stream_or_error = TcpStream::connect(&address_owned);
            if stream_or_error.is_err() 
            {
                open_connections.lock().unwrap().remove(&address_owned);
                return;
            }
            let stream = stream_or_error.unwrap();
            
            let mut send = SenderBuilder::new()
                .with_type::<Packet>()
                .with_endianness::<LittleEndian>()
                .build(BufWriter::new(stream));

            // Send them every node we know about
            send.send(&Packet::KnownNode(format!("{}:{}", THIS_NODE_ID, port))).unwrap();
            for node in &known_nodes {
                send.send(&Packet::KnownNode(node.clone())).unwrap();
            }
            send.flush().unwrap();

            Self::server(send, address_owned, recv_broadcast, open_connections)
        });

        Ok(())
    }

    fn client(server_ip: &str, mut recv: TCPReceiver, send_packet: Sender<Packet>)
    {
        loop 
        {
            match recv.recv()
            {
                Ok(Packet::Hello(mut hello)) =>
                {
                    if hello.ip == THIS_NODE_ID {
                        hello.ip = server_ip.to_owned();
                    }
                    send_packet.send(Packet::Hello(hello)).unwrap();
                },

                Ok(Packet::KnownNode(connection_data)) =>
                {
                    let data = 
                        if connection_data.starts_with(THIS_NODE_ID) {
                            connection_data.replace(THIS_NODE_ID, server_ip)
                        } else {
                            connection_data
                        };
                    send_packet.send(Packet::KnownNode(data)).unwrap();
                },

                Ok(packet) => send_packet.send(packet).unwrap(),
                Err(_) => break,
            }
        }
    }

    fn node_listener(port: i32, send_packet: Sender<Packet>)
    {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
        loop
        {
            match listener.accept()
            {
                Ok((stream, socket)) => 
                {
                    let recv = ReceiverBuilder::new()
                        .with_type::<Packet>()
                        .with_endianness::<LittleEndian>()
                        .build(BufReader::new(stream));

                    let sender = send_packet.clone();
                    std::thread::spawn(move || {
                        Self::client(&socket.ip().to_string(), recv, sender);
                    });
                },

                Err(_) => panic!(),
            }
        }
    }

    fn connect_to_known_nodes(&mut self)
    {
        if self.open_connections.lock().unwrap().len() >= self.known_nodes.len() {
            return;
        }
        
        for address in &self.known_nodes.clone() {
            self.connect(&address).unwrap();
        }
    }

    fn handle_packet(&mut self, packet: Packet)
    {
        match packet
        {
            Packet::Hello(hello) =>
            { 
                let address = format!("{}:{}", hello.ip, hello.port);
                if self.other_nodes.insert(address.clone(), hello).is_none()
                {
                    self.broadcaster.broadcast((Some( address ), Packet::Hello(OtherNode
                    {
                        ip: THIS_NODE_ID.to_owned(),
                        port: self.port,
                        top: self.my_top,
                    })));
                }
            },

            Packet::KnownNode(address) => self.update_known_nodes(&address),
            Packet::Ping => println!("Ping!!"),
            packet => self.packet_queue.push(packet),
        }
    }

    pub fn broadcast(this: &mut Arc<Mutex<Self>>, address: Option<String>, packet: Packet)
    {
        let mut this_lock = this.lock().unwrap();
        this_lock.broadcaster.broadcast((address, packet));
    }

    pub fn set_top(this: &mut Arc<Mutex<Self>>, top: u64)
    {
        let mut this_lock = this.lock().unwrap();
        let port = this_lock.port;
        this_lock.my_top = top;
        this_lock.broadcaster.broadcast((None, Packet::Hello(OtherNode 
        {
            ip: THIS_NODE_ID.to_owned(),
            port: port,
            top: top,
        })));
    }

    pub fn nodes(this: &mut Arc<Mutex<Self>>) -> HashMap<String, OtherNode>
    {
        this.lock().unwrap().other_nodes.clone()
    }

    pub fn process_packets(this: &mut Arc<Mutex<Self>>) -> Vec<Packet>
    {
        let mut this_lock = this.lock().unwrap();
        let packets = this_lock.packet_queue.clone();
        this_lock.packet_queue.clear();

        packets
    }

    pub fn run(this: Arc<Mutex<Self>>)
    {
        let (send_packet, recv_packet) = channel::<Packet>();

        {
            let mut this_lock = this.lock().unwrap();
            this_lock.connect_to_known_nodes();
            
            let port = this_lock.port;
            std::thread::spawn(move || {
                Self::node_listener(port, send_packet);
            });
        }

        std::thread::spawn(move || 
        {
            loop
            {
                match recv_packet.recv_timeout(Duration::from_secs(1))
                {
                    Ok(packet) => this.lock().unwrap().handle_packet(packet),
                    Err(mpsc::RecvTimeoutError::Timeout) => {},
                    Err(_) => break,
                }

                this.lock().unwrap().connect_to_known_nodes();
            }
        });
    }

}
