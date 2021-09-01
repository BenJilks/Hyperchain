use super::{BlockChain, BlockValidationResult, BlockChainAddResult};
use crate::block::Block;
use crate::logger::Logger;

use std::error::Error;
use std::io::Write;

#[derive(Debug, PartialEq)]
pub enum BlockChainCanMergeResult
{
    Ok,
    Empty,
    Above,
    Short,
    Invalid(BlockValidationResult),
}

impl BlockChain
{

    pub fn can_merge_branch(&mut self, branch: &[Block]) 
        -> Result<BlockChainCanMergeResult, Box<dyn Error>>
    {
        if branch.is_empty() {
            return Ok(BlockChainCanMergeResult::Empty);
        }

        // Can't attach to chain
        let bottom = branch.first().unwrap();
        if bottom.block_id > self.blocks.next_top() as u64 {
            return Ok(BlockChainCanMergeResult::Above);
        }

        // Not longer then the current branch
        let top = branch.last().unwrap();
        if top.block_id < self.blocks.next_top() as u64 {
            return Ok(BlockChainCanMergeResult::Short);
        }

        // Validate branch
        let mut last_block_or_none = 
            if bottom.block_id == 0 {
                None
            } else {
                self.block(bottom.block_id - 1)
            };

        for block in branch
        {
            if last_block_or_none.is_some()
            {
                let last_block = last_block_or_none.unwrap();

                // FIXME: Validate transactions in this case
                let (sample_start, sample_end) = self.take_sample_of_branch_at(branch, last_block.block_id);
                match block.validate_next(&last_block)?
                {
                    BlockValidationResult::Ok => {},
                    result => return Ok(BlockChainCanMergeResult::Invalid(result)),
                }
                match block.validate_content(sample_start, sample_end)?
                {
                    BlockValidationResult::Ok => {},
                    result => return Ok(BlockChainCanMergeResult::Invalid(result)),
                }
            }

            last_block_or_none = Some( block.clone() );
        }
 
        Ok(BlockChainCanMergeResult::Ok)
    }

    pub fn merge_branch<W>(&mut self, branch: Vec<Block>, logger: &mut Logger<W>)
        where W: Write
    {
        assert_eq!(self.can_merge_branch(&branch).unwrap(), BlockChainCanMergeResult::Ok);

        let bottom = branch.first().unwrap();
        self.metadata.truncate(bottom.block_id);
        self.blocks.truncate(bottom.block_id);

        for block in branch {
            assert_eq!(self.add(&block, logger).unwrap(), BlockChainAddResult::Ok);
        }
    }

}

