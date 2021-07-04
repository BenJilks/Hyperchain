mod network;
use network::{PacketHandler, ConnectionManager, Packet};
use crate::logger::{Logger, LoggerLevel};
use crate::block::{BlockChain, BlockChainAddResult};
use std::io::Write;
use std::sync::{Arc, Mutex};

pub struct Node<W>
    where W: Write + Clone + Sync + Send + 'static
{
    port: u16,
    chain: Arc<Mutex<BlockChain>>,
    logger: Logger<W>,
}

impl<W> Node<W>
    where W: Write + Clone + Sync + Send + 'static
{

    pub fn new(port: u16, chain: Arc<Mutex<BlockChain>>, logger: Logger<W>) -> Self
    {
        Self
        {
            port,
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
                match self.chain.lock().unwrap().add(&block, &mut self.logger)
                {
                    // Relay this block to the rest of the network
                    BlockChainAddResult::Ok =>
                    {
                        self.logger.log(LoggerLevel::Info, &format!("[{}] Added block {}", self.port, block.block_id));
                        connection_manager.send(Packet::Block(block.clone()));
                    },
                    
                    BlockChainAddResult::MoreNeeded(id) =>
                    {
                        self.logger.log(LoggerLevel::Info, &format!("[{}] Need block {}", self.port, id));
                        connection_manager.send_to(Packet::BlockRequest(id), |x| x == from);
                    },
                    
                    BlockChainAddResult::Duplicate => 
                        self.logger.log(LoggerLevel::Verbose, &format!("[{}] Duplicate block {}", self.port, block.block_id)),
                };
            },

            Packet::BlockRequest(id) =>
            {
                self.logger.log(LoggerLevel::Info, 
                    &format!("Got request for block {}", id));

                let chain_lock = self.chain.lock().unwrap();
                let block = chain_lock.block(id);
                if !block.is_none() {
                    connection_manager.send_to(Packet::Block(block.unwrap().clone()), |x| x == from);
                }
            },
            
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
        let node = Node::new(port, chain.clone(), logger.clone());
        let network_connection = NetworkConnection::new(port, node, logger);
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
    fn test_node_branched_chain()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let (chain_a, mut node_a) = create_node(8030, logger.clone());
        let block_a = mine_block(&chain_a, &mut node_a, &wallet, &mut logger);
        let block_b = mine_block(&chain_a, &mut node_a, &wallet, &mut logger);
        let block_c = mine_block(&chain_a, &mut node_a, &wallet, &mut logger);
        mine_block(&chain_a, &mut node_a, &wallet, &mut logger);
        mine_block(&chain_a, &mut node_a, &wallet, &mut logger);

        let (chain_b, mut node_b) = create_node(8031, logger.clone());
        chain_b.lock().unwrap().add(&block_a, &mut logger);
        chain_b.lock().unwrap().add(&block_b, &mut logger);
        chain_b.lock().unwrap().add(&block_c, &mut logger);
        mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        let block_f_b =  mine_block(&chain_b, &mut node_b, &wallet, &mut logger);

        node_b.sender().register_node("127.0.0.1:8030", None);
        let block_f_a = wait_for_block(&chain_a, 6);
        assert_eq!(block_f_a, block_f_b);
    }

    #[test]
    fn test_node_join_with_longer_chain()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let (chain_a, mut node_a) = create_node(8020, logger.clone());
        mine_block(&chain_a, &mut node_a, &wallet, &mut logger);
        mine_block(&chain_a, &mut node_a, &wallet, &mut logger);

        let (chain_b, mut node_b) = create_node(8021, logger.clone());
        mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        let block_d_on_b = mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        
        node_b.sender().register_node("127.0.0.1:8020", None);
        let block_d_on_a = wait_for_block(&chain_a, 4);
        assert_eq!(block_d_on_a, block_d_on_b);
    }

    #[test]
    fn test_node()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let (chain_a, mut node_a) = create_node(8010, logger.clone());
        let (chain_b, mut node_b) = create_node(8011, logger.clone());
        node_b.sender().register_node("127.0.0.1:8010", None);

        // Transfer block a -> b
        let block_a_on_a = mine_block(&chain_a, &mut node_a, &wallet, &mut logger);
        let block_a_on_b = wait_for_block(&chain_b, 1);
        assert_eq!(block_a_on_a, block_a_on_b);

        // Transfer 3 blocks b -> a
        let block_b_on_b = mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        let block_c_on_b = mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        let block_d_on_b = mine_block(&chain_b, &mut node_b, &wallet, &mut logger);
        let block_b_on_a = wait_for_block(&chain_a, 2);
        let block_c_on_a = wait_for_block(&chain_a, 3);
        let block_d_on_a = wait_for_block(&chain_a, 4);
        assert_eq!(block_b_on_b, block_b_on_a);
        assert_eq!(block_c_on_b, block_c_on_a);
        assert_eq!(block_d_on_b, block_d_on_a);

        // New node joins with a different, shorter chain
        let (chain_c, mut node_c) = create_node(8012, logger.clone());
        mine_block(&chain_c, &mut node_c, &wallet, &mut logger);
        mine_block(&chain_c, &mut node_c, &wallet, &mut logger);
        mine_block(&chain_c, &mut node_c, &wallet, &mut logger);
        node_c.sender().register_node("127.0.0.1:8010", None);
        let block_d_on_c = wait_for_block(&chain_c, 4);
        assert_eq!(block_d_on_c, block_d_on_a);

        // New node joins with a different, longer chain
        let (chain_d, mut node_d) = create_node(8013, logger.clone());
        mine_block(&chain_d, &mut node_d, &wallet, &mut logger);
        mine_block(&chain_d, &mut node_d, &wallet, &mut logger);
        mine_block(&chain_d, &mut node_d, &wallet, &mut logger);
        mine_block(&chain_d, &mut node_d, &wallet, &mut logger);
        let block_e_on_d = mine_block(&chain_d, &mut node_d, &wallet, &mut logger);

        node_d.sender().register_node("127.0.0.1:8010", None);
        let block_e_on_a = wait_for_block(&chain_a, 5);
        assert_eq!(block_e_on_a, block_e_on_d);
    }

}
