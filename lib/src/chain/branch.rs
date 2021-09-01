use super::{BlockChain, BlockValidationResult, BlockChainAddResult};
use crate::block::{Block, Hash};
use crate::logger::Logger;
use crate::wallet::WalletStatus;

use std::error::Error;
use std::io::Write;
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

    fn validate_branch(&mut self, branch: &[Block])
        -> Result<BlockValidationResult, Box<dyn Error>>
    {
        let bottom = branch.first().unwrap();
        let last_block_id = 
            if bottom.block_id == 0 {
                0
            } else {
                bottom.block_id - 1
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
                let new_status = block.update_wallet_status(&address, status.clone());
                if new_status.is_none() || new_status.as_ref().unwrap().balance < 0.0 {
                    return Ok(BlockValidationResult::Balance(address));
                }
                *status = new_status.unwrap();
            }

            if last_block_or_none.is_some()
            {
                let last_block = last_block_or_none.unwrap();

                // FIXME: Validate transactions in this case
                let (sample_start, sample_end) = self.take_sample_of_branch_at(branch, last_block.block_id);
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
        if bottom.block_id > self.blocks.next_top() as u64 {
            return Ok(BlockChainCanMergeResult::Above);
        }

        // Not longer then the current branch
        let top = branch.last().unwrap();
        if top.block_id < self.blocks.next_top() as u64 {
            return Ok(BlockChainCanMergeResult::Short);
        }

        // Validate branch
        match self.validate_branch(branch)?
        {
            BlockValidationResult::Ok => Ok(BlockChainCanMergeResult::Ok),
            result => Ok(BlockChainCanMergeResult::Invalid(result)),
        }
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

