use crate::node::broadcast::Broadcaster;
use crate::logger::{Logger, LoggerLevel};
use crate::block::Block;
use std::net::{TcpListener, TcpStream};
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use std::fs::File;
use std::sync::mpsc::{self, channel, Sender, Receiver};
use std::sync::{Mutex, Arc};
use std::thread::JoinHandle;
use std::time::Duration;
use std::collections::HashSet;
use tcp_channel::{ReceiverBuilder, ChannelRecv};
use tcp_channel::{SenderBuilder, ChannelSend};
use tcp_channel::LittleEndian;
use serde::{Serialize, Deserialize};

pub const THIS_NODE_ID: &str = "<<THIS NODE>>";

type TCPReceiver = tcp_channel::Receiver<Packet, LittleEndian>;
type TCPSender = tcp_channel::Sender<Packet, LittleEndian>;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Packet
{
    OnConnected(String),
    KnownNode(String),
    Block(Block),
    Ping,
}

pub struct NetworkConnection<W: Write + Clone + Sync + Send + 'static>
{
    port: i32,
    known_nodes: HashSet<String>,
    know_nodes_path: PathBuf,
    open_connections: Arc<Mutex<HashSet<String>>>,
    broadcaster: Broadcaster<(Option<String>, Packet)>,

    logger: Logger<W>,
    thread: Option<JoinHandle<()>>,
    should_shut_down: bool,
}

impl<W: Write + Clone + Sync + Send + 'static> NetworkConnection<W>
{

    pub fn new(port: i32, know_nodes_path: PathBuf, logger: Logger<W>) -> std::io::Result<Arc<Mutex<Self>>>
    {
        let known_nodes: HashSet<String> = 
            if know_nodes_path.exists() {
                serde_json::from_reader(File::open(&know_nodes_path)?).unwrap()
            } else {
                HashSet::new()
            };

        Ok(Arc::from(Mutex::from(Self
        {
            port,
            known_nodes,
            know_nodes_path,
            open_connections: Arc::from(Mutex::from(HashSet::new())),
            broadcaster: Broadcaster::new(),

            logger,
            thread: None,
            should_shut_down: false,
        })))
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
        let address_owned = 
            if address.starts_with("localhost") {
                address.replace("localhost", "127.0.0.1")
            } else {
                address.to_owned()
            };
        
        if address_owned == format!("127.0.0.1:{}", self.port) {
            return;
        }

        if self.known_nodes.contains(&address_owned) {
            return;
        }

        self.known_nodes.insert(address_owned);
        let mut file = File::create(&self.know_nodes_path).unwrap();
        file.write(&serde_json::to_vec(&self.known_nodes).unwrap()).unwrap();
    }

    fn connect(&mut self, address: &str) -> std::io::Result<()>
    {
        self.update_known_nodes(address);
        
        // If this connection is already open, ignore it
        {
            let mut open_connections_lock = self.open_connections.lock().unwrap();
            if open_connections_lock.contains(address) {
                return Ok(());
            }
            open_connections_lock.insert(address.to_owned());
        }
        
        self.logger.log(LoggerLevel::Info, 
            &format!("[{}] Connecting to {}", self.port, address));
            
        let port = self.port;
        let open_connections = self.open_connections.clone();
        let recv_broadcast = self.broadcaster.make_receiver();
        let known_nodes = self.known_nodes.clone();
        let mut logger = self.logger.clone();
        
        let address_owned = address.to_owned();
        std::thread::spawn(move || 
        {
            let stream_or_error = TcpStream::connect(&address_owned);
            if stream_or_error.is_err() 
            {
                open_connections.lock().unwrap().remove(&address_owned);
                logger.log(LoggerLevel::Warning,
                    &format!("[{}] Unable to connect to {}", port, &address_owned));
                return;
            }
            let stream = stream_or_error.unwrap();
            
            let mut send = SenderBuilder::new()
                .with_type::<Packet>()
                .with_endianness::<LittleEndian>()
                .build(BufWriter::new(stream));

            // Send them every node we know about
            send.send(&Packet::KnownNode(format!("{}:{}", THIS_NODE_ID, port))).unwrap();
            for node in &known_nodes 
            {
                if node != &address_owned 
                {
                    logger.log(LoggerLevel::Info, 
                        &format!("[{}] Sending node {} to {}", port, node, &address_owned));

                    send.send(&Packet::KnownNode(node.clone())).unwrap();
                }
            }
            send.send(&Packet::OnConnected(format!("{}:{}", THIS_NODE_ID, port))).expect("Can send");
            send.flush().expect("Can flush");

            logger.log(LoggerLevel::Info,
                &format!("[{}] Connected to {}", port, &address_owned));
            Self::server(send, address_owned, recv_broadcast, open_connections)
        });

        Ok(())
    }

    fn client(server_ip: &str, mut recv: TCPReceiver, send_packet: Sender<Packet>)
    {
        let precess_address = |address: String|
        {
            if address.starts_with(THIS_NODE_ID) {
                address.replace(THIS_NODE_ID, server_ip)
            } else {
                address
            }
        };

        loop 
        {
            match recv.recv()
            {
                Ok(Packet::KnownNode(address)) => 
                    send_packet.send(Packet::KnownNode(precess_address(address))).unwrap(),
                
                Ok(Packet::OnConnected(address)) => 
                    send_packet.send(Packet::OnConnected(precess_address(address))).unwrap(),

                Ok(packet) => 
                    send_packet.send(packet).unwrap(),
                    
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

    fn handle_packet(&mut self, packet: Packet, send_to_packet_handler: &Sender<Packet>)
    {
        self.logger.log(LoggerLevel::Info, 
            &format!("[{}] Got packet {:?}", self.port, packet));

        match packet
        {
            Packet::KnownNode(address) => 
                self.update_known_nodes(&address),

            packet => 
            {
                send_to_packet_handler
                    .send(packet)
                    .expect("Handled packet");
            },
        }
    }

    pub fn broadcast(this: &mut Arc<Mutex<Self>>, address: Option<String>, packet: Packet)
    {
        let mut this_lock = this.lock().unwrap();
        let port = this_lock.port;
        this_lock.logger.log(LoggerLevel::Info, 
            &format!("[{}] Broadcasting {:?} to {:?}", port, packet, address));

        this_lock.broadcaster.broadcast((address, packet));
    }

    pub fn run(this: Arc<Mutex<Self>>) -> Receiver<Packet>
    {
        let (send_packet, recv_packet) = channel::<Packet>();
        let (send_to_packet_handler, packet_handler) = channel::<Packet>();
        
        {
            let mut this_lock = this.lock().unwrap();
            this_lock.connect_to_known_nodes();
            
            let port = this_lock.port;
            std::thread::spawn(move || {
                Self::node_listener(port, send_packet);
            });
        }
        
        let thread_this = this.clone();
        let thread = std::thread::spawn(move ||
        {
            loop
            {
                match recv_packet.recv_timeout(Duration::from_secs(1))
                {
                    Ok(packet) => 
                        thread_this.lock().unwrap().handle_packet(packet, &send_to_packet_handler),

                    Err(mpsc::RecvTimeoutError::Timeout) => 
                    {
                        if thread_this.lock().unwrap().should_shut_down {
                            break;
                        }
                    },
                    
                    Err(err) => 
                    {
                        thread_this.lock().unwrap().logger.log(
                            LoggerLevel::Error, &format!("{:?}", err));
                        break;
                    },
                }

                thread_this.lock().unwrap().connect_to_known_nodes();
            }
        });

        this.lock().unwrap().thread = Some ( thread );
        packet_handler
    }

    pub fn shutdown(this: &Arc<Mutex<Self>>)
    {
        let thread;
        {
            let mut this_lock = this.lock().unwrap();
            if this_lock.thread.is_none() {
                return;
            }

            this_lock.logger.log(LoggerLevel::Info, 
                "Shutting down network connection...");

            this_lock.should_shut_down = true;
            thread = this_lock.thread.take().unwrap();
        }

        thread.join().expect("Join network thread");
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::logger::StdLoggerOutput;

    #[test]
    fn test_network()
    {
        let logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);

        let create_connection = |port: i32|
        {
            let connection = NetworkConnection::new(port, 
                std::env::temp_dir().join(format!("{}.json", rand::random::<u32>())), logger.clone())
                .expect("Create connection");
            connection.lock().unwrap().update_known_nodes("localhost:8000");
            let recv = NetworkConnection::run(connection.clone());

            (connection, recv)
        };

        let (mut connection_a, recv_a) = create_connection(8000);
        let (mut connection_b, recv_b) = create_connection(8001);
        let (_, recv_c) = create_connection(8002);

        let recv_on_connect_packets = |recv: &Receiver<Packet>, ports: &[i32]|
        {
            let mut packets = Vec::<Packet>::new();
            for _ in 0..ports.len() {
                packets.push(recv.recv().expect("Got packet"));
            }

            for port in ports {
                assert_eq!(packets.contains(&Packet::OnConnected(format!("127.0.0.1:{}", port))), true);
            }
        };

        recv_on_connect_packets(&recv_a, &[8001, 8002]);
        recv_on_connect_packets(&recv_b, &[8000, 8002]);
        recv_on_connect_packets(&recv_c, &[8000, 8001]);

        NetworkConnection::broadcast(&mut connection_a, None, Packet::Ping);
        NetworkConnection::broadcast(&mut connection_b, None, Packet::Ping);
        assert_eq!(recv_a.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_b.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);

        let (mut connection_d, recv_d) = create_connection(8003);
        recv_on_connect_packets(&recv_a, &[8003]);
        recv_on_connect_packets(&recv_b, &[8003]);
        recv_on_connect_packets(&recv_c, &[8003]);
        recv_on_connect_packets(&recv_d, &[8000, 8001, 8002]);

        NetworkConnection::broadcast(&mut connection_d, None, Packet::Ping);
        assert_eq!(recv_a.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_b.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);
    }

}
