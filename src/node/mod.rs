pub mod network;
mod broadcast;
use network::{NetworkConnection, Packet};
use crate::logger::{Logger, LoggerLevel};
use crate::block::{Block, BlockChain};

use std::sync::{Mutex, Arc};
use std::sync::mpsc::RecvTimeoutError;
use std::boxed::Box;
use std::thread::JoinHandle;
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;

pub struct Node<W: Write + Clone + Sync + Send + 'static>
{
    connection: Arc<Mutex<NetworkConnection<W>>>,
    chain: BlockChain,
    logger: Logger<W>,
    thread: Option<JoinHandle<()>>,
    should_shut_down: bool,
}

impl<W: Write + Clone + Sync + Send + 'static> Node<W>
{

    pub fn new(port: i32, know_nodes_path: PathBuf, mut logger: Logger<W>) -> Arc<Mutex<Self>>
    {
        let connection = NetworkConnection::new(port, know_nodes_path, logger.clone())
            .expect("Create connection");

        Arc::from(Mutex::from(Self
        {
            connection,
            chain: BlockChain::new(&mut logger),
            logger,
            thread: None,
            should_shut_down: false,
        }))
    }

    fn handle_packet(&mut self, packet: Packet)
    {
        match packet
        {
            Packet::KnownNode(_) => {},

            Packet::OnConnected(address) => 
            {
                let top = self.chain.top();
                if !top.is_none() 
                {
                    NetworkConnection::broadcast(&mut self.connection, 
                        Some( address ), Packet::Block(top.unwrap().clone()));
                }
            },

            Packet::Block(block) =>
            {
                // Try and add the block to the chain, if it's a duplicate, ignore it
                if self.chain.add(&block, &mut self.logger) 
                {
                    // Relay this block to the rest of the network
                    NetworkConnection::broadcast(&mut self.connection, 
                        None, Packet::Block(block.clone()));
                }
            }
            
            Packet::Ping => 
                self.logger.log(LoggerLevel::Info, "Ping!"),
        }
    }

    pub fn add_block(this: &mut Arc<Mutex<Self>>, block: &Block)
    {
        let mut this_lock = this.lock().unwrap();
        let mut logger = this_lock.logger.clone();
        if !this_lock.chain.add(block, &mut logger) {
            return;
        }

        NetworkConnection::broadcast(&mut this_lock.connection, 
            None, Packet::Block(block.clone()));
    }

    pub fn add_known_node(this: &mut Arc<Mutex<Self>>, address: &str)
    {
        let this_lock = this.lock().unwrap();
        let mut connection_lock = this_lock.connection.lock().unwrap();
        connection_lock.update_known_nodes(address);
    }

    pub fn run(this: Arc<Mutex<Self>>)
    {
        let thread_this = this.clone();
        let thread = std::thread::spawn(move ||
        {
            let recv = NetworkConnection::run(thread_this.lock().unwrap().connection.clone());
            loop
            {
                match recv.recv_timeout(Duration::from_millis(100))
                {
                    Ok(packet) => 
                        thread_this.lock().unwrap().handle_packet(packet),
                    
                    Err(RecvTimeoutError::Timeout) => 
                    {
                        if thread_this.lock().unwrap().should_shut_down {
                            break;
                        }
                    },

                    Err(_) => 
                        break,
                }
            }

            NetworkConnection::shutdown(&thread_this.lock().unwrap().connection);
        });

        this.lock().unwrap().thread = Some ( thread );
    }

    pub fn shutdown(this: Arc<Mutex<Self>>)
    {
        let thread;
        {
            let mut this_lock = this.lock().unwrap();
            if this_lock.thread.is_none() {
                return;
            }

            this_lock.logger.log(LoggerLevel::Info, "Shutting down node...");
            this_lock.should_shut_down = true;
            thread = this_lock.thread.take().unwrap();
        }

        thread.join().expect("Joined nodes thread");
    }

}

#[cfg(test)]
pub mod tests
{

    use super::*;
    use crate::logger::{Logger, LoggerLevel, StdLoggerOutput};
    use crate::wallet::PrivateWallet;
    use crate::miner;
    use std::time::Duration;

    fn wait_for_block<W>(node: &Arc<Mutex<Node<W>>>, block_id: u64) -> Block
        where W: Write + Clone + Sync + Send + 'static
    {
        loop
        {
            {
                let chain = &node.lock().unwrap().chain;
                let block_or_none = chain.block(block_id);
                if block_or_none.is_some() {
                    return block_or_none.unwrap().clone();
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn create_node<W>(port: i32, logger: Logger<W>) -> Arc<Mutex<Node<W>>>
        where W: Write + Clone + Sync + Send + 'static
    {
        let temp_file = std::env::temp_dir().join(format!("{}.json", rand::random::<u32>()));
        let mut node = Node::new(port, temp_file, logger);
        Node::add_known_node(&mut node, "127.0.0.1:8010");
        Node::run(node.clone());

        node
    }

    fn mine_block<W>(node: &mut Arc<Mutex<Node<W>>>, wallet: &PrivateWallet) -> Block
        where W: Write + Clone + Sync + Send + 'static
    {
        let block;
        {
            let chain = &mut node.lock().unwrap().chain;
            block = miner::mine_block(Block::new(&chain, wallet).expect("Create block"));
        }
        Node::add_block(node, &block);

        block
    }

    #[test]
    pub fn test_node()
    {
        let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let mut node_a = create_node(8010, logger.clone());
        let mut node_b = create_node(8011, logger.clone());

        // mine block a on a
        let block_a_on_a = mine_block(&mut node_a, &wallet);
        let block_a_on_b = wait_for_block(&node_b, 1);
        assert_eq!(block_a_on_a, block_a_on_b);

        // mine block b on b
        let block_b_on_b = mine_block(&mut node_b, &wallet);
        let block_b_on_a = wait_for_block(&node_a, 2);
        assert_eq!(block_b_on_a, block_b_on_b);

        Node::shutdown(node_a);
        Node::shutdown(node_b);
    }

}
