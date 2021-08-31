pub mod network;
use network::{PacketHandler, ConnectionManager, Packet};
use crate::logger::{Logger, LoggerLevel};
use crate::chain::{BlockChain, BlockChainAddResult, BlockChainCanMergeResult};
use crate::block::Block;
use crate::block::validate::BlockValidate;

use std::io::Write;
use std::error::Error;
use std::collections::HashMap;

pub struct Node<W>
    where W: Write + Clone + Sync + Send + 'static
{
    port: u16,
    chain: BlockChain,
    logger: Logger<W>,
    branches: HashMap<String, Vec<Block>>,
}

impl<W> Node<W>
    where W: Write + Clone + Sync + Send + 'static
{

    pub fn new(port: u16, chain: BlockChain, logger: Logger<W>) -> Self
    {
        Self
        {
            port,
            chain,
            logger,
            branches: HashMap::new(),
        }
    }

    pub fn chain(&mut self) -> &mut BlockChain
    {
        &mut self.chain
    }

    fn is_valid_next_entry_in_branch(branch: &Vec<Block>, block: &Block) -> bool
    {
        if branch.is_empty() {
            return true;
        }

        let bottom = branch.first().unwrap();
        bottom.validate_next(block).is_ok()
    }

    fn add_to_branch(&mut self, from: &str, block: Block)
    {
        if !self.branches.contains_key(from) {
            self.branches.insert(from.to_owned(), Vec::new());
        }

        let mut branch = self.branches.remove(from).unwrap();
        if Self::is_valid_next_entry_in_branch(&branch, &block) 
        {
            branch.insert(0, block);
            self.branches.insert(from.to_owned(), branch);
        }
    }

    fn complete_branch(&mut self, from: &str) -> Result<(), Box<dyn Error>>
    {
        if !self.branches.contains_key(from) {
            return Ok(());
        }

        let branch = self.branches.remove(from).unwrap();
        if self.chain.can_merge_branch(&branch)? == BlockChainCanMergeResult::Ok
        {
            self.logger.log(LoggerLevel::Info, &format!("[{}] Merge longer branch", self.port));
            self.chain.merge_branch(branch);
        }
        Ok(())
    }

}

impl<W> PacketHandler<W> for Node<W>
    where W: Write + Clone + Sync + Send + 'static
{

    fn on_packet(&mut self, from: &str, packet: Packet, connection_manager: &mut ConnectionManager<W>)
        -> Result<(), Box<dyn Error>>
    {
        match packet
        {
            Packet::KnownNode(_) => {},

            Packet::OnConnected(_) => 
            {
                match self.chain.top()
                {
                    Some(top) => 
                    {
                        connection_manager.send_to(Packet::Block(top.clone()), 
                            |addr| addr == from);
                    },
                    None => {},
                }
            },

            Packet::Block(block) =>
            {
                // Try and add the block to the chain, if it's a duplicate or invalid, ignore it
                match self.chain.add(&block)?
                {
                    BlockChainAddResult::Ok =>
                    {
                        // Relay this block to the rest of the network
                        self.logger.log(LoggerLevel::Info, &format!("[{}] Added block {}", self.port, block.block_id));
                        connection_manager.send(Packet::Block(block.clone()));
                    },

                    BlockChainAddResult::Invalid(_) | BlockChainAddResult::MoreNeeded => 
                    {
                        self.logger.log(LoggerLevel::Info, &format!("[{}] Invalid block {}", self.port, block.block_id));

                        let block_id = block.block_id;
                        self.add_to_branch(from, block);
                        if block_id == 0 {
                            self.complete_branch(from)?;
                        } else {
                            connection_manager.send_to(Packet::BlockRequest(block_id - 1), |x| x == from);
                        }
                    },

                    BlockChainAddResult::Duplicate => 
                    {
                        self.logger.log(LoggerLevel::Verbose, &format!("[{}] Duplicate block {}", self.port, block.block_id));
                        self.complete_branch(from)?;
                    },
                }
            },

            Packet::BlockRequest(id) =>
            {
                self.logger.log(LoggerLevel::Info, 
                    &format!("Got request for block {}", id));

                let block = self.chain.block(id);
                if block.is_some() {
                    connection_manager.send_to(Packet::Block(block.unwrap().clone()), |x| x == from);
                }
            },
            
            Packet::Ping => 
                self.logger.log(LoggerLevel::Info, "Ping!"),
        }

        Ok(())
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use network::NetworkConnection;
    use crate::logger::{Logger, LoggerLevel, StdLoggerOutput};
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::block::Block;
    use crate::miner;

    use std::time::Duration;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn wait_for_block<W>(connection: &Arc<Mutex<NetworkConnection<Node<W>, W>>>, block_id: u64) -> Block
        where W: Write + Clone + Sync + Send + 'static
    {
        loop
        {
            {
                let mut connection_lock = connection.lock().unwrap();
                let chain = connection_lock.handler().chain();
                let block_or_none = chain.block(block_id);
                if block_or_none.is_some() {
                    return block_or_none.unwrap().clone();
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn create_node<W>(port: u16, mut logger: Logger<W>) -> Arc<Mutex<NetworkConnection<Node<W>, W>>>
        where W: Write + Clone + Sync + Send + 'static
    {
        let chain = BlockChain::new(&mut logger);
        let node = Node::new(port, chain, logger.clone());
        let network_connection = NetworkConnection::new(port, node, logger);
        network_connection
    }

    fn mine_block<W>(connection: &Arc<Mutex<NetworkConnection<Node<W>, W>>>, 
                     wallet: &PrivateWallet) -> Block
        where W: Write + Clone + Sync + Send + 'static
    {
        let mut connection_lock = connection.lock().unwrap();
        let chain = &mut connection_lock.handler().chain();
        let block = miner::mine_block(Block::new(chain, wallet)
            .expect("Create block"));

        chain.add(&block).unwrap();
        connection_lock.manager().send(Packet::Block(block.clone()));

        block
    }

    #[test]
    fn test_node_branched_chain()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let mut connection_a = create_node(8030, logger.clone());
        let block_a = mine_block(&mut connection_a, &wallet);
        let block_b = mine_block(&mut connection_a, &wallet);
        let block_c = mine_block(&mut connection_a, &wallet);
        mine_block(&mut connection_a, &wallet);
        mine_block(&mut connection_a, &wallet);

        let mut connection_b = create_node(8031, logger.clone());
        {
            let mut connection_b_lock = connection_b.lock().unwrap();
            connection_b_lock.handler().chain().add(&block_a).unwrap();
            connection_b_lock.handler().chain().add(&block_b).unwrap();
            connection_b_lock.handler().chain().add(&block_c).unwrap();
        }
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        let block_h_b = mine_block(&mut connection_b, &wallet);

        connection_b.lock().unwrap().manager().register_node("127.0.0.1:8030", None);
        let block_h_a = wait_for_block(&connection_a, 7);
        assert_eq!(block_h_a, block_h_b);
    }

    #[test]
    fn test_node_join_with_longer_chain()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let mut connection_a = create_node(8020, logger.clone());
        mine_block(&mut connection_a, &wallet);
        mine_block(&mut connection_a, &wallet);

        let mut connection_b = create_node(8021, logger.clone());
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        let block_d_on_b = mine_block(&mut connection_b, &wallet);
        
        connection_b.lock().unwrap().manager().register_node("127.0.0.1:8020", None);
        let block_d_on_a = wait_for_block(&connection_a, 3);
        assert_eq!(block_d_on_a, block_d_on_b);
    }

    #[test]
    fn test_node()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let mut connection_a = create_node(8010, logger.clone());
        let mut connection_b = create_node(8011, logger.clone());
        connection_b.lock().unwrap().manager().register_node("127.0.0.1:8010", None);

        // Transfer block a -> b
        let block_a_on_a = mine_block(&mut connection_a, &wallet);
        let block_a_on_b = wait_for_block(&connection_b, 0);
        assert_eq!(block_a_on_a, block_a_on_b);

        // Transfer 3 blocks b -> a
        let block_b_on_b = mine_block(&mut connection_b, &wallet);
        let block_c_on_b = mine_block(&mut connection_b, &wallet);
        let block_d_on_b = mine_block(&mut connection_b, &wallet);
        let block_e_on_b = mine_block(&mut connection_b, &wallet);
        let block_b_on_a = wait_for_block(&connection_a, 1);
        let block_c_on_a = wait_for_block(&connection_a, 2);
        let block_d_on_a = wait_for_block(&connection_a, 3);
        let block_e_on_a = wait_for_block(&connection_a, 4);
        assert_eq!(block_b_on_b, block_b_on_a);
        assert_eq!(block_c_on_b, block_c_on_a);
        assert_eq!(block_d_on_b, block_d_on_a);
        assert_eq!(block_e_on_b, block_e_on_a);

        // New node joins with a different, shorter chain
        let mut connection_c = create_node(8012, logger.clone());
        mine_block(&mut connection_c, &wallet);
        mine_block(&mut connection_c, &wallet);
        mine_block(&mut connection_c, &wallet);
        connection_c.lock().unwrap().manager().register_node("127.0.0.1:8010", None);
        let block_e_on_c = wait_for_block(&connection_c, 4);
        assert_eq!(block_e_on_c, block_e_on_a);

        // New node joins with a different, longer chain
        let mut connection_d = create_node(8013, logger.clone());
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        let block_f_on_d = mine_block(&mut connection_d, &wallet);

        connection_d.lock().unwrap().manager().register_node("127.0.0.1:8010", None);
        let block_f_on_a = wait_for_block(&connection_a, 5);
        assert_eq!(block_f_on_a, block_f_on_d);
    }

}

