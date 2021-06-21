pub mod network;
mod broadcast;
use network::NetworkConnection;

use std::sync::{Mutex, Arc};
use std::io::Write;

pub struct Node<W: Write + Clone + Sync + Send + 'static>
{
    connection: Arc<Mutex<NetworkConnection<W>>>,
}
