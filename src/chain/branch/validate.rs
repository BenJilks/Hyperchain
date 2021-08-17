use super::Branch;
use super::add::{CanAddResult, BranchAdd};
use crate::logger::{Logger, LoggerLevel};
use crate::block::{Block, HASH_LEN};
use crate::block::validate::BlockValidate;
use crate::block::transactions::BlockTransactions;
use crate::block::target::BLOCK_SAMPLE_SIZE;
use crate::wallet::WalletStatus;

use std::collections::HashMap;
use std::io::Write;

pub trait BranchValidate
{
    fn remove_invalid_blocks(&mut self, logger: &mut Logger<impl Write>) -> Vec<Branch>;
}

impl Branch
{

    fn split_branch(&mut self, at: u64) -> Vec<Branch>
    {
        let mut new_branches = Vec::<Branch>::new();
        let new_branch_bottom = self.bottom;

        // Make this branch the top split
        self.blocks.remove(&at);
        self.bottom = at + 1;
        if at == new_branch_bottom {
            return new_branches;
        }

        // Create new branch for the bottom split
        let new_branch_top = self.blocks.remove(&(at - 1)).unwrap();
        let mut new_branch = Branch::new(new_branch_top);
        for i in (new_branch_bottom..=(at - 2)).rev() 
        {
            let block = self.blocks.remove(&i).unwrap();
            assert_eq!(new_branch.try_add(&block), CanAddResult::Yes);
        }

        // Move sub-branches to new branch if needed
        let keys = self.sub_branches.keys().map(|x| *x).collect::<Vec<i32>>();
        for sub_branch_id in keys
        {
            let sub_branch_bottom = self.sub_branches[&sub_branch_id].bottom;
            if sub_branch_bottom <= at 
            {
                let sub_branch = self.sub_branches.remove(&sub_branch_id).unwrap();

                // NOTE: If this branches from the block we're removing, we 
                //       can't add this sub-branch to either top or bottom.
                if sub_branch_bottom == at {
                    new_branches.push(sub_branch);
                } else {
                    new_branch.sub_branches.insert(sub_branch_id, sub_branch);
                }
            }
        }

        new_branches.push(new_branch);
        new_branches
    }

    fn is_block_valid(&self, account_status: &mut HashMap<[u8; HASH_LEN], WalletStatus>, 
                      block: &Block, next: Option<&Block>) -> bool
    {
        // Check final balance is valid
        for address in block.get_addresses_used()
        {
            if !account_status.contains_key(&address) {
                account_status.insert(address.clone(), WalletStatus::default());
            }

            let status = account_status.get_mut(&address).unwrap();
            block.update_wallet_status(&address, status);
            
            if !status.is_valid() {
                return false;
            }
        }

        // Check internal block data
        if block.block_id <= BLOCK_SAMPLE_SIZE
        {
            if !block.is_valid(None, None) {
                return false;
            }
        }
        else
        {
            let sample_end = self.block(block.block_id - 1);
            let sample_start = self.block(block.block_id - 1 - BLOCK_SAMPLE_SIZE);
            if !block.is_valid(sample_start, sample_end) {
                return false;
            }
        }

        // Check the next block is correct, if there is one
        if next.is_some()
        {
            if next.unwrap().is_next_block(block).is_err() {
                return false;
            }
        }

        true
    }

    // Returns a list of sub branches that branch of from the block id specified
    fn sub_branch_ids_for_block(&self, block_id: u64) -> Vec<i32>
    {
        let mut branches = Vec::<i32>::new();
        for (sub_branch_id, sub_branch) in &self.sub_branches
        {
            if sub_branch.bottom == block_id {
                branches.push(*sub_branch_id);
            }
        }

        branches
    }

    fn remove_invalid_blocks_impl(&mut self, root_account_status: Option<&HashMap<[u8; HASH_LEN], WalletStatus>>,
                                  logger: &mut Logger<impl Write>) -> Vec<Branch>
    {
        let mut new_branches = Vec::<Branch>::new();
        let mut account_status = 
            if root_account_status.is_none() {
                HashMap::<[u8; HASH_LEN], WalletStatus>::new()
            } else {
                root_account_status.unwrap().clone()
            };

        for id in self.bottom..=self.top
        {
            let block = &self.blocks[&id];
            let next = 
                if id == self.top {
                    None 
                } else {
                    Some( &self.blocks[&(id + 1)] )
                };

            if !self.is_block_valid(&mut account_status, block, next) 
            {
                logger.log(LoggerLevel::Warning, 
                    &format!("Removing invalid block {}", id));

                new_branches.append(&mut self.split_branch(id));
                for sub_branch_id in self.sub_branch_ids_for_block(id) {
                    new_branches.push(self.sub_branches.remove(&sub_branch_id).unwrap());
                }
                break;
            }

            for sub_branch_id in self.sub_branch_ids_for_block(id) 
            {
                let sub_branch = self.sub_branches.get_mut(&sub_branch_id).unwrap();
                let mut sub_new_branches = sub_branch.remove_invalid_blocks_impl(Some( &account_status ), logger);
                new_branches.append(&mut sub_new_branches);
            }
        }

        new_branches
    }

}

impl BranchValidate for Branch
{

    fn remove_invalid_blocks(&mut self, logger: &mut Logger<impl Write>) -> Vec<Branch>
    {
        if !self.is_complete() {
            Vec::new()
        } else {
            self.remove_invalid_blocks_impl(None, logger)
        }
    }

}

