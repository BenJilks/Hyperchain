pub mod network;
use network::{PacketHandler, ConnectionManager, Packet};
use crate::logger::{Logger, LoggerLevel};
use crate::block::BlockChain;
use std::io::Write;
use std::sync::{Arc, Mutex};

pub struct Node<W>
    where W: Write + Clone + Sync + Send + 'static
{
    chain: Arc<Mutex<BlockChain>>,
    logger: Logger<W>,
}

impl<W> Node<W>
    where W: Write + Clone + Sync + Send + 'static
{

    pub fn new(chain: Arc<Mutex<BlockChain>>, logger: Logger<W>) -> Self
    {
        Self
        {
            chain,
            logger,
        }
    }

}

impl<W> PacketHandler<W> for Node<W>
    where W: Write + Clone + Sync + Send + 'static
{

    fn on_packet(&mut self, from: &str, packet: Packet, connection_manager: &mut ConnectionManager<W>)
    {
        match packet
        {
            Packet::KnownNode(_) => {},

            Packet::OnConnected(_) => 
            {
                let chain_lock = self.chain.lock().unwrap();
                let top = chain_lock.top();
                if !top.is_none() 
                {
                    connection_manager.send_to(Packet::Block(top.unwrap().clone()), 
                        |addr| addr == from);
                }
            },

            Packet::Block(block) =>
            {
                // Try and add the block to the chain, if it's a duplicate, ignore it
                if self.chain.lock().unwrap().add(&block, &mut self.logger) 
                {
                    // Relay this block to the rest of the network
                    connection_manager.send(Packet::Block(block.clone()));
                }
            }
            
            Packet::Ping => 
                self.logger.log(LoggerLevel::Info, "Ping!"),
        }
    }

}

#[cfg(test)]
pub mod tests
{

    use super::*;
    use network::NetworkConnection;
    use crate::logger::{Logger, LoggerLevel, StdLoggerOutput};
    use crate::wallet::PrivateWallet;
    use crate::block::Block;
    use crate::miner;
    use std::time::Duration;
    use std::path::PathBuf;

    fn wait_for_block(chain: &Arc<Mutex<BlockChain>>, block_id: u64) -> Block
    {
        loop
        {
            {
                let chain_lock = chain.lock().unwrap();
                let block_or_none = chain_lock.block(block_id);
                if block_or_none.is_some() {
                    return block_or_none.unwrap().clone();
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn create_node<W>(port: u16, mut logger: Logger<W>) -> (Arc<Mutex<BlockChain>>, NetworkConnection<W>)
        where W: Write + Clone + Sync + Send + 'static
    {
        // let temp_file = std::env::temp_dir().join(format!("{}.json", rand::random::<u32>()));
        let chain = Arc::from(Mutex::from(BlockChain::new(&mut logger)));
        let node = Node::new(chain.clone(), logger.clone());
        let mut network_connection = NetworkConnection::new(port, node, logger);
        network_connection.sender().register_node("127.0.0.1:8010", None);

        (chain, network_connection)
    }

    fn mine_block<W>(chain: &Arc<Mutex<BlockChain>>, node: &mut NetworkConnection<W>, 
                     wallet: &PrivateWallet, logger: &mut Logger<W>) -> Block
        where W: Write + Clone + Sync + Send + 'static
    {
        let block;
        {
            let mut chain_lock = chain.lock().unwrap();
            block = miner::mine_block(Block::new(&chain_lock, wallet).expect("Create block"));
            chain_lock.add(&block, logger);
        }
        node.sender().send(Packet::Block(block.clone()));

        block
    }

    #[test]
    pub fn test_node()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let (chain_a, mut node_a) = create_node(8010, logger.clone());
        let (chain_b, mut node_b) = create_node(8011, logger.clone());

        // mine block a on a
        let block_a_on_a = mine_block(&chain_a, &mut node_a, &wallet, &mut logger);
        let block_a_on_b = wait_for_block(&chain_b, 1);
        assert_eq!(block_a_on_a, block_a_on_b);

        // mine block b on b
        let block_b_on_b = mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        let block_b_on_a = wait_for_block(&chain_a, 2);
        assert_eq!(block_b_on_a, block_b_on_b);
    }

}
