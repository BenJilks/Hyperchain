use super::Node;
use crate::network::packet::{Packet, PacketHandler};
use crate::network::client_manager::ClientManager;

use std::sync::{Arc, Mutex, MutexGuard};
use std::error::Error;

#[derive(Clone)]
pub struct NodePacketHandler
{
    node: Arc<Mutex<Node>>,
}

impl NodePacketHandler
{

    pub fn new(node: Arc<Mutex<Node>>) -> Self
    {
        Self
        {
            node,
        }
    }

    pub fn node(&self) -> MutexGuard<Node>
    {
        self.node.lock().unwrap()
    }

}

impl PacketHandler for NodePacketHandler
{

    fn handle(&self, from: &str, packet: Packet, manager: &mut ClientManager)
        -> Result<(), Box<dyn Error>>
    {
        let mut node = self.node.lock().unwrap();
        match packet
        {
            Packet::OnConnected => 
            {
                if let Some(top) = node.chain.top()
                {
                    manager.send_to(Packet::Block(top.clone()),
                        |addr| addr == from)?;
                }

                manager.send_to(Packet::Report(None, node.our_report()?),
                    |addr| addr == from)?;
            },

            Packet::Block(block) => 
                node.handle_block(manager, from, block)?,

            Packet::BlockRequest(id) =>
                node.handle_block_request(manager, from, id)?,

            Packet::Transfer(transfer) =>
                node.handle_transfer(manager, from, transfer)?,

            Packet::Page(page, data) =>
                node.handle_page(manager, from, page, data)?,

            Packet::Report(address, report) =>
                match address
                {
                    Some(addr) => node.handle_report(manager, &addr, report)?,
                    None => node.handle_report(manager, from, report)?,
                },

            Packet::Ping(time_sent) =>
                manager.report_ping_time(from, time_sent),
        }

        Ok(())
    }

    fn update_reports(&self, manager: &mut ClientManager)
    {
        let mut node = self.node.lock().unwrap();
        node.update_reports(manager);
    }

}

