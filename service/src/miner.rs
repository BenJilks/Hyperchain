use crate::network::NetworkConnection;
use crate::network::packet::Packet;
use crate::node::packet_handler::NodePacketHandler;
use crate::block_builder;

use libhyperchain::block::Block;
use libhyperchain::block::validate::BlockValidationResult;
use libhyperchain::block;
use libhyperchain::chain::BlockChainAddResult;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use std::path::PathBuf;
use std::thread::JoinHandle;
use std::error::Error;

pub fn mine_block_unless_found(connection: &NetworkConnection<NodePacketHandler>, 
                               mut block: Block) 
    -> Result<Block, Box<dyn Error>>
{
    while block.validate_pow()? != BlockValidationResult::Ok
    { 
        block.header.pow += 1;

        // Check this block wasn't already mined
        if block.header.pow % 100 == 0
        {
            // Delay for testing
            std::thread::sleep(std::time::Duration::from_millis(10));

            if connection.should_shutdown() {
                break;
            }

            let mut node = connection.handler().node();
            let chain = node.chain();
            if chain.block(block.header.block_id).is_some() {
                break;
            }
        }
    }

    Ok(block)
}

fn mine_next_block(connection: &mut NetworkConnection<NodePacketHandler>,
                   wallet: &PrivateWallet) -> Result<(), Box<dyn Error>>
{
    let mut block;
    {
        // Create the next block
        let mut node = connection.handler().node();
        let chain = &mut node.chain();
        block = block_builder::build(chain, wallet)?;
    }

    // Do the mining work
    block = mine_block_unless_found(connection, block)?;
    if block.validate_pow()? != BlockValidationResult::Ok {
        return Ok(());
    }

    // Add it to the chain if it's still the top
    let handler = connection.handler().clone();
    let mut node = handler.node();
    let chain = &mut node.chain();

    let top = chain.top();
    if top.is_none() || top.unwrap().header.block_id + 1 == block.header.block_id 
    {
        match chain.add(&block)?
        {
            BlockChainAddResult::Ok =>
            {
                println!("Won block {}! With difficulty {}", 
                    block.header.block_id, 
                    block::target::difficulty(&block.header.target));

                let data = node.data_store().for_page_updates(&block.pages)?;
                connection.manager().send(Packet::Block(block, data))?;
            },

            _ => {},
        }
    }

    Ok(())
}

pub fn start_miner_thread(mut connection: NetworkConnection<NodePacketHandler>) 
    -> JoinHandle<()>
{
    // Create chain a wallet
    let wallet = PrivateWallet::read_from_file(&PathBuf::from("test.wallet")).unwrap();

    std::thread::spawn(move || loop 
    {
        mine_next_block(&mut connection, &wallet).unwrap();
        if connection.should_shutdown() {
            break;
        }
    })
}

