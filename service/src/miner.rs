use crate::block::Block;
use crate::block::validate::BlockValidate;
use crate::node::network::NetworkConnection;
use crate::node::Node;

use std::sync::{Arc, Mutex};
use std::io::Write;

pub fn mine_block(mut block: Block) -> Block
{
    while !block.is_pow_valid() {
        block.pow += 1;
    }

    block
}

pub fn mine_block_unless_found<W>(network_connection: &Arc<Mutex<NetworkConnection<Node<W>, W>>>, 
                                  mut block: Block) -> Block
    where W: Write + Clone + Sync + Send + 'static
{
    while !block.is_pow_valid() 
    { 
        block.pow += 1;

        // Check this block wasn't already mined
        if block.pow % 100 == 0
        {
            // Delay for testing
            std::thread::sleep(std::time::Duration::from_millis(10));

            let mut network_connection_lock = network_connection.lock().unwrap();
            if network_connection_lock.should_shutdown() {
                break;
            }

            let chain = &network_connection_lock.handler().chain();
            if chain.block(block.block_id).is_some() {
                break;
            }
        }
    }

    block
}
