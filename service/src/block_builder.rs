use libhyperchain::block::Block;
use libhyperchain::chain::BlockChain;
use libhyperchain::wallet::Wallet;
use std::error::Error;

pub fn build<W>(chain: &BlockChain, wallet: &W) -> Result<Block, Box<dyn Error>>
    where W: Wallet
{
    // FIXME: Validate transactions
    let mut block = Block::new(chain, wallet)?;
    for transaction in chain.get_next_transactions_in_queue(10) {
        block.add_transaction(transaction.clone());
    }

    Ok(block)
}

