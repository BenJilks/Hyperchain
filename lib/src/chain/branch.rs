use super::{BlockChain, BLOCK_SAMPLE_SIZE};
use super::{BlockValidationResult, BlockChainAddResult};
use crate::block::Block;
use crate::wallet::WalletStatus;
use crate::hash::Hash;

use std::error::Error;
use std::collections::HashMap;

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

    fn take_sample_of_branch_at(&mut self, branch: &[Block], block_id: u64) 
        -> (Option<Block>, Option<Block>)
    {
        assert_eq!(branch.is_empty(), false);

        if block_id < BLOCK_SAMPLE_SIZE {
            return (None, None);
        }

        let branch_start = branch.first().unwrap();
        let mut block_at = |block_id: u64| -> Option<Block>
        {
            if block_id >= branch_start.header.block_id 
            {
                match branch.get((block_id - branch_start.header.block_id) as usize)
                {
                    Some(block) => Some(block.clone()),
                    None => None,
                }
            } 
            else 
            {
                self.block(block_id)
            }
        };

        let sample_start = block_at(block_id - BLOCK_SAMPLE_SIZE);
        let sample_end = block_at(block_id);
        (sample_start, sample_end)
    }

    pub fn validate_branch(&mut self, branch: &[Block])
        -> Result<BlockValidationResult, Box<dyn Error>>
    {
        let bottom = branch.first().unwrap();
        let last_block_id = 
            if bottom.header.block_id == 0 {
                0
            } else {
                bottom.header.block_id - 1
            };

        let mut last_block_or_none = self.block(last_block_id);
        let mut wallets = HashMap::<Hash, WalletStatus>::new();
        for block in branch
        {
            for address in block.get_addresses_used()
            {
                if !wallets.contains_key(&address) 
                {
                    let status = self.get_wallet_status_up_to_block(last_block_id, &address);
                    wallets.insert(address, status);
                }

                let status = wallets.get_mut(&address).unwrap();
                let new_status = block.update_wallet_status(&address, status.clone())?;
                if new_status.balance < 0.0 {
                    return Ok(BlockValidationResult::Balance(address));
                }
                *status = new_status;
            }

            if last_block_or_none.is_some()
            {
                let last_block = last_block_or_none.unwrap();

                // FIXME: Validate transactions in this case
                let (sample_start, sample_end) = self.take_sample_of_branch_at(branch, last_block.header.block_id);
                match block.validate_next(&last_block)?
                {
                    BlockValidationResult::Ok => {},
                    result => return Ok(result),
                }

                match block.validate_content(sample_start, sample_end)?
                {
                    BlockValidationResult::Ok => {},
                    result => return Ok(result),
                }
            }

            last_block_or_none = Some( block.clone() );
        }
 
        Ok(BlockValidationResult::Ok)
    }

    pub fn can_merge_branch(&mut self, branch: &[Block]) 
        -> Result<BlockChainCanMergeResult, Box<dyn Error>>
    {
        if branch.is_empty() {
            return Ok(BlockChainCanMergeResult::Empty);
        }

        // Can't attach to chain
        let bottom = branch.first().unwrap();
        if bottom.header.block_id > self.blocks.next_top() as u64 {
            return Ok(BlockChainCanMergeResult::Above);
        }

        // Not longer then the current branch
        let top = branch.last().unwrap();
        if top.header.block_id < self.blocks.next_top() as u64 {
            return Ok(BlockChainCanMergeResult::Short);
        }

        // Validate branch
        match self.validate_branch(branch)?
        {
            BlockValidationResult::Ok => Ok(BlockChainCanMergeResult::Ok),
            result => Ok(BlockChainCanMergeResult::Invalid(result)),
        }
    }

    pub fn merge_branch(&mut self, branch: Vec<Block>)
    {
        assert_eq!(self.can_merge_branch(&branch).unwrap(), BlockChainCanMergeResult::Ok);

        let bottom = branch.first().unwrap();
        self.metadata.truncate(bottom.header.block_id);
        self.blocks.truncate(bottom.header.block_id);

        for block in branch {
            assert_eq!(self.add(&block).unwrap(), BlockChainAddResult::Ok);
        }
    }

}
