use super::packet::{Packet, PacketHandler};
use super::client_manager::ClientManager;

use std::net::TcpStream;
use std::thread::JoinHandle;
use std::time::Duration;
use std::error::Error;

fn try_connect_to_node<H>(address: String, packet_handler: &H,
                          manager: &mut ClientManager)
    -> Result<(), Box<dyn Error>>
    where H: PacketHandler + Clone + Send + Sync + 'static
{
    debug!("[{}] Trying to connect to {}", manager.port(), address);

    let sock_address = address.parse()?;
    let stream = TcpStream::connect_timeout(&sock_address, Duration::from_secs(1))?;
    let ip = sock_address.ip().to_string();
    manager.new_client(packet_handler.clone(), stream, ip)?;
    Ok(())
}

fn connect_to_new_nodes<H>(packet_handler: &H, 
                           manager: &mut ClientManager)
    where H: PacketHandler + Clone + Send + Sync + 'static
{
    let not_connected_nodes = manager.get_not_connected_nodes();
    for address in not_connected_nodes {
        let _ = try_connect_to_node(address, packet_handler, manager);
    }
}

fn ping_old_nodes(manager: &mut ClientManager)
{
    let _ = manager.send(Packet::Ping);
}

pub fn start_node_discovery_thread<H>(packet_handler: H, 
                                      mut manager: ClientManager)
    -> JoinHandle<()>
    where H: PacketHandler + Clone + Send + Sync + 'static
{
    std::thread::spawn(move || loop
    {
        if manager.should_shutdown() 
        {
            debug!("[{}] Exit node discovery", manager.port());
            break;
        }

        connect_to_new_nodes(&packet_handler, &mut manager);
        ping_old_nodes(&mut manager);

        std::thread::sleep(Duration::from_millis(1000));
    })
}
