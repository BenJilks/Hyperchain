use super::client_manager::ClientManager;
use libhyperchain::block::Block;
use libhyperchain::data_store::DataUnit;
use libhyperchain::transaction::Transaction;
use libhyperchain::transaction::transfer::Transfer;
use libhyperchain::transaction::page::Page;
use libhyperchain::config::Hash;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::error::Error;

pub type MessageSender = tcp_channel::Sender<Message, tcp_channel::LittleEndian>;
pub type MessageReceiver = tcp_channel::Receiver<Message, tcp_channel::LittleEndian>;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Packet
{
    OnConnected,
    Block(Block, HashMap<Hash, DataUnit>),
    BlockRequest(u64),
    Transfer(Transaction<Transfer>),
    Page(Transaction<Page>, DataUnit),
    Ping,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Message
{
    OnConnected(u16),
    KnownNode(String),
    Packet(Packet),
}

pub trait PacketHandler
{
    fn handle(&self, from: &str, packet: Packet, manager: &mut ClientManager)
        -> Result<(), Box<dyn Error>>;
}

