use libhyperchain::block::Block;
use libhyperchain::chain::BlockChain;
use libhyperchain::wallet::Wallet;
use std::error::Error;

pub fn build<W>(chain: &mut BlockChain, wallet: &W) -> Result<Block, Box<dyn Error>>
    where W: Wallet
{
    // FIXME: Validate transfer
    let mut block = Block::new(chain, wallet)?;
    for transfer in chain.get_next_transfers_in_queue(10) {
        block.add_transfer(transfer.clone());
    }
    for page in chain.get_next_pages_in_queue(10) {
        block.add_page(page.clone());
    }

    Ok(block)
}
