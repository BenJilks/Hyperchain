use crate::block::Block;
use crate::block::validate::BlockValidationResult;

pub fn mine_block(mut block: Block) -> Block
{
    while block.validate_pow().unwrap() != BlockValidationResult::Ok {
        block.pow += 1;
    }

    block
}

