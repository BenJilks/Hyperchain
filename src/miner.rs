use crate::block::Block;
use sha2::Sha256;

pub fn mine_block(mut block: Block) -> Block
{
    let mut hasher = Sha256::default();
    while !block.validate_pow_with_hasher(&mut hasher) {
        block.pow += 1;
    }

    block
}
