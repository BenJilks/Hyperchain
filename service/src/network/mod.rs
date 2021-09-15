pub mod packet;
pub mod client;
pub mod server;
pub mod client_manager;
mod node_discovery;
use packet::PacketHandler;
use client_manager::ClientManager;
use server::start_server_thread;
use node_discovery::start_node_discovery_thread;

use std::net::TcpStream;
use std::thread::JoinHandle;
use std::sync::{Arc, Mutex};
use std::error::Error;

struct NetworkConnectionData
{
    port: u16,
    shutdown_signal: Arc<Mutex<bool>>,
    server_thread: Option<JoinHandle<()>>,
    node_discovery_thread: Option<JoinHandle<()>>,
    manager: ClientManager,
}

#[derive(Clone)]
pub struct NetworkConnection<H>
    where H: PacketHandler
{
    data: Arc<Mutex<NetworkConnectionData>>,
    shutdown_signal: Arc<Mutex<bool>>,
    handler: H,
    manager: ClientManager,
}

impl<H> NetworkConnection<H>
    where H: PacketHandler + Clone + Send + Sync + 'static
{

    pub fn open(port: u16, packet_handler: H) -> Result<Self, Box<dyn Error>>
    {
        let shutdown_signal = Arc::from(Mutex::from(false));
        let manager = ClientManager::new(port, shutdown_signal.clone());

        let server = start_server_thread(
            packet_handler.clone(), manager.clone())?;
        
        let node_discovery = start_node_discovery_thread(
            packet_handler.clone(), manager.clone());

        Ok(Self
        {
            data: Arc::from(Mutex::from(NetworkConnectionData
            {
                port,
                shutdown_signal: shutdown_signal.clone(),
                server_thread: Some(server),
                node_discovery_thread: Some(node_discovery),
                manager: manager.clone(),
            })),

            shutdown_signal,
            handler: packet_handler,
            manager,
        })
    }

    pub fn should_shutdown(&self) -> bool
    {
        *self.shutdown_signal.lock().unwrap()
    }

    pub fn manager(&mut self) -> &mut ClientManager
    {
        &mut self.manager
    }

    pub fn handler(&self) -> &H
    {
        &self.handler
    }

}

impl Drop for NetworkConnectionData
{

    fn drop(&mut self)
    {
        info!("[{}] Shutting down connection", self.port);

        *self.shutdown_signal.lock().unwrap() = true;
        self.manager.shutdown();
        let _ = TcpStream::connect(format!("127.0.0.1:{}", self.port));

        let server = self.server_thread.take().unwrap();
        server.join().unwrap();

        let node_discovery = self.node_discovery_thread.take().unwrap();
        node_discovery.join().unwrap();
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::packet::Packet;

    use std::sync::mpsc::{Sender, Receiver, channel};
    use std::error::Error;

    #[derive(Clone)]
    struct TestCommandHandler
    {
        test_sender: Arc<Mutex<Sender<Packet>>>,
    }

    impl PacketHandler for TestCommandHandler
    {

        fn handle(&self, _: &str, packet: Packet, _: &mut ClientManager)
            -> Result<(), Box<dyn Error>>
        {
            let _ = self.test_sender.lock().unwrap().send(packet);
            Ok(())
        }

    }

    fn create_connection(port: u16) -> (NetworkConnection<TestCommandHandler>, Receiver<Packet>)
    {
        let (send, recv) = channel();
        let command_handler = TestCommandHandler { test_sender: Arc::from(Mutex::from(send)) };
        let connection = NetworkConnection::open(port, command_handler).unwrap();

        (connection, recv)
    }

    #[test]
    fn test_network_disconnect()
    {
        let _ = pretty_env_logger::try_init();

        let (mut connection_a, recv_a) = create_connection(8180);
        {
            let (mut connection_b, recv_b) = create_connection(8181);
            connection_b.manager().register_node("127.0.0.1:8180");
            assert_eq!(recv_a.recv().unwrap(), Packet::OnConnected);
            assert_eq!(recv_b.recv().unwrap(), Packet::OnConnected);

            connection_a.manager().send(Packet::Ping).unwrap();
            assert_eq!(recv_b.recv().unwrap(), Packet::Ping);

            // NOTE: Disconnects here
        }

        let (_connection_b, recv_b) = create_connection(8181);
        assert_eq!(recv_a.recv().unwrap(), Packet::OnConnected);
        assert_eq!(recv_b.recv().unwrap(), Packet::OnConnected);

        connection_a.manager().send(Packet::Ping).unwrap();
        assert_eq!(recv_b.recv().unwrap(), Packet::Ping);
    }

    #[test]
    fn test_network()
    {
        let _ = pretty_env_logger::try_init();

        let (mut connection_a, recv_a) = create_connection(8000);
        let (mut connection_b, recv_b) = create_connection(8001);
        let (mut connection_c, recv_c) = create_connection(8002);
        connection_b.manager().register_node("127.0.0.1:8000");
        connection_c.manager().register_node("127.0.0.1:8000");

        let recv_on_connect_command = |recv: &Receiver<Packet>, count: usize|
        {
            let mut connection_count = 0;
            while connection_count < count
            {
                match recv.recv_timeout(std::time::Duration::from_secs(10))
                {
                    Ok(Packet::OnConnected) => connection_count += 1,
                    Ok(Packet::Ping) => {},
                    _ => panic!(),
                }
            }
        };

        recv_on_connect_command(&recv_a, 2);
        recv_on_connect_command(&recv_b, 2);
        recv_on_connect_command(&recv_c, 2);

        connection_a.manager().send(Packet::Ping).unwrap();
        connection_b.manager().send(Packet::Ping).unwrap();
        assert_eq!(recv_a.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_b.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);

        let (mut connection_d, recv_d) = create_connection(8003);
        connection_d.manager().register_node("127.0.0.1:8000");
        recv_on_connect_command(&recv_a, 1);
        recv_on_connect_command(&recv_b, 1);
        recv_on_connect_command(&recv_c, 1);
        recv_on_connect_command(&recv_d, 3);

        connection_d.manager().send(Packet::Ping).unwrap();
        assert_eq!(recv_a.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_b.recv().expect("Got packet"), Packet::Ping);
        assert_eq!(recv_c.recv().expect("Got packet"), Packet::Ping);
    }

}

