use crate::logger::{LoggerLevel, Logger};
use crate::block::Block;
use std::io::{Write, BufReader, BufWriter};
use std::net::{TcpStream, TcpListener};
use std::thread::JoinHandle;
use std::sync::mpsc::{channel, Sender, Receiver, RecvTimeoutError};
use std::sync::{Mutex, MutexGuard, Arc};
use std::collections::{HashSet, HashMap};
use std::time::Duration;
use tcp_channel::{ReceiverBuilder, ChannelRecv};
use tcp_channel::{SenderBuilder, ChannelSend};
use tcp_channel::LittleEndian;
use serde::{Serialize, Deserialize};

type TcpReceiver<T> = tcp_channel::Receiver<T, LittleEndian>;
type TcpSender<T> = tcp_channel::Sender<T, LittleEndian>;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub enum Packet
{
    KnownNode(String),
    OnConnected(u16),
    Block(Block),
    BlockRequest(u64),
    Ping,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message
{
    Packet(String, Packet),
    Shutdown,
}

fn start_packet_reciver<W>(server_ip: String, mut recv: TcpReceiver<Packet>, 
                           message_sender: Sender<Message>, mut logger: Logger<W>) -> JoinHandle<()>
    where W: Write + Sync + Send + 'static
{
    std::thread::spawn(move || loop
    {
        match recv.recv()
        {
            Ok(packet) =>
            {
                match message_sender.send(Message::Packet(server_ip.clone(), packet)) 
                {
                    Ok(_) => {},
                    Err(err) => 
                    {
                        logger.log(LoggerLevel::Error, 
                            &format!("message_sender.send(packet): {}", err));
                        break;
                    },
                }
            },

            Err(tcp_channel::RecvError::IoError(e)) 
                if e.kind() == std::io::ErrorKind::UnexpectedEof ||
                   e.kind() == std::io::ErrorKind::ConnectionReset =>
            {
                // The stream has closed
                break;
            },
            
            Err(err) =>
            {
                logger.log(LoggerLevel::Error, &format!("recv.recv(): {}", err));
                break;
            },
        }
    })
}

fn start_node_listner<P, W>(port: u16, network_connection: Arc<Mutex<NetworkConnection<P, W>>>,
                            should_shutdown: Arc<Mutex<bool>>) -> JoinHandle<()>
    where P: PacketHandler<W> + Sync + Send + 'static,
          W: Write + Clone + Sync + Send + 'static
{
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();
    std::thread::spawn(move || loop
    {
        match listener.accept()
        {
            Ok((stream, socket)) =>
            {
                let mut network_connection_lock = network_connection.lock().unwrap();
                let address = format!("{}:{}", socket.ip(), socket.port());
                network_connection_lock.logger.log(LoggerLevel::Info, 
                    &format!("[{}] Got connection request from {}", port, address));

                if *should_shutdown.lock().unwrap() {
                    break;
                }

                network_connection_lock.manager().add_client(address, stream);
            },

            Err(_) => 
            {
                let mut network_connection_lock = network_connection.lock().unwrap();
                network_connection_lock.logger.log(LoggerLevel::Info, 
                    &format!("[{}] Shutdown node listner", port));

                break;
            },
        }
    })
}

pub trait PacketHandler<W>
    where W: Write + Clone + Sync + Send + 'static
{
    fn on_packet(&mut self, from: &str, packet: Packet, connection_manager: &mut ConnectionManager<W>);
}

fn handle_message_packet<P, W>(from: String, packet: Packet, 
                               network_connection: &mut NetworkConnection<P, W>)
    where P: PacketHandler<W> + Sync + Send + 'static,
          W: Write + Clone + Sync + Send + 'static
{
    let port = network_connection.port;

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
                network_connection.handler().on_packet(&from, packet, &mut manager_lock)
            }
        },

        _ => 
        {
            let manager = network_connection.connection_manager.clone().unwrap();
            let mut manager_lock = manager.lock().unwrap();
            network_connection.handler().on_packet(&from, packet, &mut manager_lock)
        },
    }
}

fn start_message_handler<P, W>(network_connection: Arc<Mutex<NetworkConnection<P, W>>>, 
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
                network_connection_lock.logger.log(LoggerLevel::Info, &format!("[{}] Got packet", port));

                handle_message_packet(from, packet, &mut network_connection_lock);
                network_connection_lock.logger.log(LoggerLevel::Info, &format!("[{}] Handled packet", port));
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

fn start_network_connection<P, W>(network_connection: Arc<Mutex<NetworkConnection<P, W>>>)
    where P: PacketHandler<W> + Sync + Send + 'static,
          W: Write + Clone + Sync + Send + 'static
{
    let mut network_connection_lock = network_connection.lock().unwrap();

    // Create channel for recived packets to be send through
    let message_reciver = network_connection_lock.open_manager();

    // Start server for other nodes to connect to
    network_connection_lock.node_listner_thread = Some(start_node_listner(network_connection_lock.port,
        network_connection.clone(), network_connection_lock.should_shutdown.clone()));

    // Start thread to handle incoming packets
    network_connection_lock.message_handler_thread = Some(start_message_handler( 
        network_connection.clone(), message_reciver));
}

struct Connection
{
    stream: TcpStream,
    reciver_thread: Option<JoinHandle<()>>,
    sender: TcpSender<Packet>,
    public_address: Option<String>,
}

impl Connection
{

    pub fn new<W>(port: u16, address: &str, stream: TcpStream, message_sender: Sender<Message>, logger: Logger<W>) -> std::io::Result<Self>
        where W: Write + Sync + Send + 'static
    {
        let reciver = ReceiverBuilder::new()
            .with_type::<Packet>()
            .with_endianness::<LittleEndian>()
            .build(BufReader::new(stream.try_clone()?));
        let reciver_thread = start_packet_reciver(address.to_owned(), reciver, message_sender, logger);

        let mut sender = SenderBuilder::new()
            .with_type::<Packet>()
            .with_endianness::<LittleEndian>()
            .build(BufWriter::new(stream.try_clone()?));
        if sender.send(&Packet::OnConnected(port)).is_err() {
            return Err(std::io::Error::from(std::io::ErrorKind::NotConnected));
        }
        sender.flush()?;

        Ok(Self
        {
            stream,
            reciver_thread: Some( reciver_thread ),
            sender,
            public_address: None,
        })
    }

}

impl Drop for Connection
{

    fn drop(&mut self)
    {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        self.reciver_thread
            .take().unwrap()
            .join().expect("Join server connection");
    }

}

pub struct ConnectionManager<W>
    where W: Write + Clone + Sync + Send + 'static
{
    port: u16,
    message_sender: Sender<Message>,
    known_nodes: HashSet<String>,
    open_connections: HashSet<String>,
    connections: HashMap<String, Connection>,
    logger: Logger<W>
}

impl<W> ConnectionManager<W>
    where W: Write + Clone + Sync + Send + 'static
{

    pub fn new(port: u16, message_sender: Sender<Message>, logger: Logger<W>) -> Arc<Mutex<Self>>
    {
        Arc::from(Mutex::from(Self
        {
            port,
            message_sender,
            known_nodes: HashSet::new(),
            open_connections: HashSet::new(),
            connections: HashMap::new(),
            logger,
        }))
    }

    fn add_client(&mut self, address: String, stream: TcpStream)
    {
        self.logger.log(LoggerLevel::Info, 
            &format!("[{}] Connected to {}", self.port, address));

        match Connection::new(self.port, &address, stream, self.message_sender.clone(), self.logger.clone())
        {
            Ok(connection) => {
                self.connections.insert(address, connection);
            },

            _ => {},
        };
    }

    fn confirm_connection(&mut self, address: &str, public_address: String)
    {
        let connection = &mut self.connections.get_mut(address).unwrap();
        connection.public_address = Some( public_address );
    }

    pub fn register_node(&mut self, address: &str, from: Option<&str>)
    {
        if self.known_nodes.insert(address.to_owned())
        {
            self.logger.log(LoggerLevel::Verbose, 
                &format!("[{}] Regestered new node {}", self.port, address));
            
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
            self.logger.log(LoggerLevel::Warning,
                &format!("[{}] Already to connected to {}", self.port, address));
            return;
        }

        let stream_or_error = TcpStream::connect(address);
        if stream_or_error.is_err() 
        {
            self.logger.log(LoggerLevel::Verbose,
                &format!("[{}] Unable to connect to {}", self.port, address));
            return;
        }

        let stream = stream_or_error.unwrap();
        self.add_client(address.to_owned(), stream);
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

            self.logger.log(LoggerLevel::Verbose, 
                &format!("[{}] Sending {:?} to {}", self.port, packet, address));

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

impl<W> Drop for ConnectionManager<W>
    where W: Write + Clone + Sync + Send + 'static
{

    fn drop(&mut self)
    {
        self.logger.log(LoggerLevel::Info, 
            &format!("[{}] Shutting down {} connection(s)", self.port, self.connections.len()));
        self.connections.clear();
    }

}

pub struct NetworkConnection<P, W>
    where W: Write + Clone + Sync + Send + 'static,
          P: PacketHandler<W> + Sync + Send + 'static
{
    port: u16,
    should_shutdown: Arc<Mutex<bool>>,
    packet_handler: P,
    logger: Logger<W>,

    message_sender: Option<Sender<Message>>,
    connection_manager: Option<Arc<Mutex<ConnectionManager<W>>>>,
    node_listner_thread: Option<JoinHandle<()>>,
    message_handler_thread: Option<JoinHandle<()>>,
}

impl<P, W> NetworkConnection<P, W>
    where W: Write + Clone + Sync + Send + 'static,
          P: PacketHandler<W> + Sync + Send + 'static
{

    pub fn new(port: u16, packet_handler: P, logger: Logger<W>) -> Arc<Mutex<Self>>
    {
        let should_shutdown = Arc::from(Mutex::from(false));
        let network_connecton = Arc::from(Mutex::from(Self
        {
            port,
            should_shutdown,
            packet_handler,
            logger,

            message_sender: None,
            connection_manager: None,
            node_listner_thread: None,
            message_handler_thread: None,
        }));

        start_network_connection(network_connecton.clone());
        network_connecton
    }

    fn open_manager(&mut self) -> Receiver<Message>
    {
        let (message_sender, message_reciver) = channel::<Message>();
        self.message_sender = Some( message_sender.clone() );
        self.connection_manager = Some( ConnectionManager::new(self.port, 
            message_sender, self.logger.clone()) );

        message_reciver
    }

    pub fn manager(&mut self) -> MutexGuard<ConnectionManager<W>>
    {
        self.connection_manager
            .as_mut().unwrap()
            .lock().unwrap()
    }

    pub fn handler(&mut self) -> &mut P
    {
        &mut self.packet_handler
    }

}

impl<P, W> Drop for NetworkConnection<P, W>
    where W: Write + Clone + Sync + Send + 'static,
          P: PacketHandler<W> + Sync + Send + 'static
{

    fn drop(&mut self)
    {
        self.logger.log(LoggerLevel::Info, 
            &format!("[{}] Shutting down network connection", self.port));

        // Shutdown node listner
        if self.node_listner_thread.is_some()
        {
            let node_listner_thread = self.node_listner_thread.take().unwrap();
            *self.should_shutdown.lock().unwrap() = true;
            let _ = TcpStream::connect(&format!("127.0.0.1:{}", self.port));
            node_listner_thread.join().expect("Joined server thread");
        }

        // Shutdown message handler
        if self.message_handler_thread.is_some() && self.message_sender.is_some()
        {
            let message_handler_thread = self.message_handler_thread.take().unwrap();
            self.message_sender.as_mut().unwrap().send(Message::Shutdown).expect("Sent shutdown message");
            message_handler_thread.join().expect("Joined message handler thread");
        }
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::logger::StdLoggerOutput;

    struct TestPacketHandler
    {
        test_sender: Mutex<Sender<Packet>>,
    }

    impl<W> PacketHandler<W> for TestPacketHandler
        where W: Write + Clone + Sync + Send + 'static
    {

        fn on_packet(&mut self, _: &str, packet: Packet, _: &mut ConnectionManager<W>)
        {
            let _ = self.test_sender.lock().unwrap().send(packet);
        }

    }

    fn create_connection<W>(port: u16, logger: Logger<W>) -> (Arc<Mutex<NetworkConnection<TestPacketHandler, W>>>, Receiver<Packet>)
        where W: Write + Clone + Sync + Send + 'static
    {
        let (send, recv) = channel();
        let packet_handler = TestPacketHandler { test_sender: Mutex::from(send) };
        let connection = NetworkConnection::new(port, packet_handler, logger);

        (connection, recv)
    }

    #[test]
    fn test_network_disconnect()
    {
        let logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let (connection_a, recv_a) = create_connection(8080, logger.clone());
        {
            let (connection_b, _recv_b) = create_connection(8081, logger.clone());
            connection_b.lock().unwrap().manager().register_node("127.0.0.1:8080", None);
            println!("{:?}", recv_a.recv());
        }

        connection_a.lock().unwrap().manager().send(Packet::Ping);
    }

    #[test]
    fn test_network()
    {
        let logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);

        let (connection_a, recv_a) = create_connection(8000, logger.clone());
        let (connection_b, recv_b) = create_connection(8001, logger.clone());
        let (connection_c, recv_c) = create_connection(8002, logger.clone());
        connection_b.lock().unwrap().manager().register_node("127.0.0.1:8000", None);
        connection_c.lock().unwrap().manager().register_node("127.0.0.1:8000", None);

        let recv_on_connect_packets = |recv: &Receiver<Packet>, ports: &[u16]|
        {
            for _ in 0..ports.len()
            {
                match recv.recv_timeout(std::time::Duration::from_secs(10))
                {
                    Ok(Packet::OnConnected(port)) => 
                        assert_eq!(ports.contains(&port), true),

                    _ => panic!(),
                }
            }
        };

        recv_on_connect_packets(&recv_a, &[8001, 8002]);
        recv_on_connect_packets(&recv_b, &[8000, 8002]);
        recv_on_connect_packets(&recv_c, &[8001, 8000]);

        connection_a.lock().unwrap().manager().send(Packet::Ping);
        connection_b.lock().unwrap().manager().send(Packet::Ping);
        assert_eq!(recv_a.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_b.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);

        let (connection_d, recv_d) = create_connection(8003, logger.clone());
        connection_d.lock().unwrap().manager().register_node("127.0.0.1:8000", None);
        recv_on_connect_packets(&recv_a, &[8003]);
        recv_on_connect_packets(&recv_b, &[8003]);
        recv_on_connect_packets(&recv_c, &[8003]);
        recv_on_connect_packets(&recv_d, &[8000, 8001, 8002]);

        connection_d.lock().unwrap().manager().send(Packet::Ping);
        assert_eq!(recv_a.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_b.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);
    }

}
