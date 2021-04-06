use crate::block::Block;

pub fn mine_block(mut block: Block) -> Block
{
    while !block.validate_pow() {
        block.pow += 1;
    }

    block
}
