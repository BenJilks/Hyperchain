use crate::block::{Block, BlockChain};
use crate::wallet::Wallet;
use crate::logger::Logger;

use std::io::Write;

pub fn mine_block(mut block: Block) -> Block
{
    // std::thread::sleep(std::time::Duration::from_secs(1));
    while !block.is_pow_valid() {
        block.pow += 1;
    }

    block
}

pub fn mine<W: Wallet>(chain: &mut BlockChain, wallet: &W, count: i32, logger: &mut Logger<impl Write>)
{
    for _ in 0..count
    {
        let block = Block::new(chain.current_branch(), wallet).expect("Can create new block");
        chain.add(&mine_block(block), logger);
    }
}

