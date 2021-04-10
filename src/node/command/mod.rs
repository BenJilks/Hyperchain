mod transaction;
mod page;
mod balance;
pub use transaction::TransactionCommand;
pub use page::PageCommand;
pub use balance::BalanceCommand;
use crate::node::network::{NetworkConnection, Packet};
use crate::block::{Block, BlockChain};
use crate::logger::Logger;

use std::sync::{Arc, Mutex};
use std::io::Write;

pub trait Command<W: Write>
{

    fn name(&self) -> &'static str;

    fn invoke(&mut self, args: &[String], connection: &mut Arc<Mutex<NetworkConnection>>, 
        chain: &mut BlockChain, logger: &mut Logger<W>);

    fn on_packet(&mut self, _packet: Packet, _connection: &mut Arc<Mutex<NetworkConnection>>, _chain: &mut BlockChain) {}

    fn on_create_block(&mut self, _block: &mut Block) {}

    fn on_accepted_block(&mut self, _block: &Block) {}

}
