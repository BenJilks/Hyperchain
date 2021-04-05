use crate::block::{Block, BlockChainBranch};
use crate::wallet::PrivateWallet;
use crate::error::Error;

pub fn mine_block(mut block: Block) -> Block
{
    while !block.validate_pow() {
        block.pow += 1;
    }

    block
}
