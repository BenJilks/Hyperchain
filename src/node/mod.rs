mod network;
mod broadcast;
use crate::block::{Block, BlockChain, BlockChainBranch};
use crate::wallet::PrivateWallet;
use network::{NetworkConnection, Packet};

use std::sync::{Mutex, Arc};
use std::path::PathBuf;

pub struct Node
{
    connection: Arc<Mutex<NetworkConnection>>,
}

impl Node
{

    pub fn new(port: i32, known_nodes: PathBuf) -> Self
    {
        let connection = NetworkConnection::new(port, known_nodes).unwrap();
        Self
        {
            connection: Arc::from(Mutex::from(connection)),
        }
    }

}
