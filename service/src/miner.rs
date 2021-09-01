use crate::node::network::{NetworkConnection, Packet};
use crate::node::Node;
use crate::block_builder;

use libhyperchain::block::Block;
use libhyperchain::block::validate::BlockValidationResult;
use libhyperchain::block;
use libhyperchain::chain::BlockChainAddResult;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use libhyperchain::logger::Logger;
use std::sync::{Arc, Mutex};
use std::io::Write;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::error::Error;

pub fn mine_block_unless_found<W>(network_connection: &Arc<Mutex<NetworkConnection<Node<W>, W>>>, 
                                  mut block: Block) -> Result<Block, Box<dyn Error>>
    where W: Write + Clone + Sync + Send + 'static
{
    while block.validate_pow()? != BlockValidationResult::Ok
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

    Ok(block)
}

fn mine_next_block<W>(network_connection: &Arc<Mutex<NetworkConnection<Node<W>, W>>>,
                      wallet: &PrivateWallet) -> Result<(), Box<dyn Error>>
    where W: Write + Clone + Sync + Send + 'static
{
    let mut block;
    {
        // Create the next block
        let mut network_connection_lock = network_connection.lock().unwrap();
        let chain = &network_connection_lock.handler().chain();
        block = block_builder::build(chain, wallet)?;
    }

    // Do the mining work
    block = mine_block_unless_found(network_connection, block)?;
    if block.validate_pow()? != BlockValidationResult::Ok {
        return Ok(());
    }

    // Add it to the chain if it's still the top
    let mut network_connection_lock = network_connection.lock().unwrap();
    let mut logger = network_connection_lock.logger.clone();
    let chain = &mut network_connection_lock.handler().chain();
    let top = chain.top();
    if top.is_none() || top.unwrap().block_id + 1 == block.block_id 
    {
        match chain.add(&block, &mut logger)?
        {
            BlockChainAddResult::Ok =>
            {
                println!("Won block {}! With difficulty {}", 
                    block.block_id, 
                    block::target::difficulty(&block.target));

                network_connection_lock.manager().send(Packet::Block(block));
            },

            _ => {},
        }
    }

    Ok(())
}

pub fn start_miner_thread<W>(network_connection: Arc<Mutex<NetworkConnection<Node<W>, W>>>,
                             mut logger: Logger<W>) -> JoinHandle<()>
    where W: Write + Clone + Sync + Send + 'static
{
    // Create chain a wallet
    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

    std::thread::spawn(move || loop 
    {
        mine_next_block(&network_connection, &wallet).unwrap();
        if network_connection.lock().unwrap().should_shutdown() {
            break;
        }
    })
}

