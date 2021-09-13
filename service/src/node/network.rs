use super::packet_handler::{PacketHandler, Message};
use super::packet_handler::start_message_handler;
use super::manager::ConnectionManager;

use std::net::{TcpStream, TcpListener};
use std::thread::JoinHandle;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Mutex, MutexGuard, Arc};

pub struct NetworkConnection<P>
    where P: PacketHandler + Sync + Send + 'static
{
    pub(crate) port: u16,
    should_shutdown: bool,
    packet_handler: P,

    message_sender: Option<Sender<Message>>,
    pub(crate) connection_manager: Option<Arc<Mutex<ConnectionManager>>>,
    node_listner_thread: Option<JoinHandle<()>>,
    message_handler_thread: Option<JoinHandle<()>>,
}

fn start_node_listner<P>(port: u16, network_connection: Arc<Mutex<NetworkConnection<P>>>) 
        -> JoinHandle<()>
    where P: PacketHandler + Sync + Send + 'static
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
                info!("[{}] Got connection request from {}", port, address);

                if network_connection_lock.should_shutdown() {
                    break;
                }

                network_connection_lock.manager().add_client(address, stream);
            },

            Err(_) => 
            {
                info!("[{}] Shutdown node listner", port);
                break;
            },
        }
    })
}

fn start_network_connection<P>(network_connection: Arc<Mutex<NetworkConnection<P>>>)
    where P: PacketHandler + Sync + Send + 'static
{
    let mut network_connection_lock = network_connection.lock().unwrap();

    // Create channel for recived packets to be send through
    let message_reciver = network_connection_lock.open_manager();

    // Start server for other nodes to connect to
    network_connection_lock.node_listner_thread = Some(start_node_listner(network_connection_lock.port,
        network_connection.clone()));

    // Start thread to handle incoming packets
    network_connection_lock.message_handler_thread = Some(start_message_handler( 
        network_connection.clone(), message_reciver));
}

impl<P> NetworkConnection<P>
    where P: PacketHandler + Sync + Send + 'static
{

    pub fn new(port: u16, packet_handler: P) -> Arc<Mutex<Self>>
    {
        let network_connecton = Arc::from(Mutex::from(Self
        {
            port,
            should_shutdown: false,
            packet_handler,

            message_sender: None,
            connection_manager: None,
            node_listner_thread: None,
            message_handler_thread: None,
        }));

        start_network_connection(network_connecton.clone());
        network_connecton
    }

    fn signal_stop_threads(&mut self) -> Vec<JoinHandle<()>>
    {
        let mut threads_to_wait_for = Vec::<JoinHandle<()>>::new();
        info!("[{}] Shutting down network connection", self.port);

        // Shutdown node listner
        if self.node_listner_thread.is_some()
        {
            self.should_shutdown = true;
            let node_listner_thread = self.node_listner_thread.take().unwrap();
            let _ = TcpStream::connect(&format!("127.0.0.1:{}", self.port));
            threads_to_wait_for.push(node_listner_thread);
        }

        // Shutdown message handler
        if self.message_handler_thread.is_some() && self.message_sender.is_some()
        {
            let message_handler_thread = self.message_handler_thread.take().unwrap();
            self.message_sender.as_mut().unwrap().send(Message::Shutdown).expect("Sent shutdown message");
            threads_to_wait_for.push(message_handler_thread);
        }

        threads_to_wait_for
    }

    pub fn shutdown(this: &Arc<Mutex<Self>>)
    {
        let threads_to_wait_for = this
            .lock().unwrap()
            .signal_stop_threads();

        for thread in threads_to_wait_for {
            thread.join().unwrap();
        }
    }

    pub fn should_shutdown(&self) -> bool
    {
        self.should_shutdown
    }

    fn open_manager(&mut self) -> Receiver<Message>
    {
        let (message_sender, message_reciver) = channel::<Message>();
        self.message_sender = Some( message_sender.clone() );
        self.connection_manager = Some( ConnectionManager::new(self.port, 
            message_sender) );

        message_reciver
    }

    pub fn manager(&mut self) -> MutexGuard<ConnectionManager>
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

impl<P> Drop for NetworkConnection<P>
    where P: PacketHandler + Sync + Send + 'static
{

    fn drop(&mut self)
    {
        for thread in self.signal_stop_threads() {
            thread.join().unwrap();
        }
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::super::packet_handler::Packet;

    use std::error::Error;

    struct TestPacketHandler
    {
        test_sender: Mutex<Sender<Packet>>,
    }

    impl PacketHandler for TestPacketHandler
    {

        fn on_packet(&mut self, _: &str, packet: Packet, _: &mut ConnectionManager)
            -> Result<(), Box<dyn Error>>
        {
            let _ = self.test_sender.lock().unwrap().send(packet);
            Ok(())
        }

    }

    fn create_connection(port: u16) -> (Arc<Mutex<NetworkConnection<TestPacketHandler>>>, Receiver<Packet>)
    {
        let (send, recv) = channel();
        let packet_handler = TestPacketHandler { test_sender: Mutex::from(send) };
        let connection = NetworkConnection::new(port, packet_handler);

        (connection, recv)
    }

    #[test]
    fn test_network_disconnect()
    {
        let (connection_a, recv_a) = create_connection(8080);
        {
            let (connection_b, _recv_b) = create_connection(8081);
            connection_b.lock().unwrap().manager().register_node("127.0.0.1:8080", None);
            println!("{:?}", recv_a.recv());
        }

        connection_a.lock().unwrap().manager().send(Packet::Ping);
    }

    #[test]
    fn test_network()
    {
        let (connection_a, recv_a) = create_connection(8000);
        let (connection_b, recv_b) = create_connection(8001);
        let (connection_c, recv_c) = create_connection(8002);
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

        let (connection_d, recv_d) = create_connection(8003);
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
