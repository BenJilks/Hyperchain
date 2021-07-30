pub mod prune;
use crate::logger::{Logger, LoggerLevel};
use crate::block::{Block, HASH_LEN};
use crate::wallet::WalletStatus;

use std::collections::HashMap;
use std::fmt;
use std::io::Write;

pub struct Branch
{
    blocks: HashMap<u64, Block>,
    sub_branches: HashMap<i32, Branch>,
    top: u64,
    bottom: u64,
}

#[derive(PartialEq, Debug)]
pub enum CanAddResult
{
    Yes,
    InSubBranch(i32),
    Duplicate,
    Invalid,
}

#[derive(PartialEq)]
enum MergeType
{
    Extends,
    OtherIsSubBranch,
    WeAreSubChain,
    MergeSubBranch(i32),
    NoMerge,
}

impl Branch
{

    pub fn new(block: Block) -> Self
    {
        let start_id = block.block_id;
        let mut blocks = HashMap::<u64, Block>::new();
        blocks.insert(start_id, block);

        Branch
        {
            blocks,
            sub_branches: HashMap::new(),
            top: start_id,
            bottom: start_id,
        }
    }

    pub fn can_add(&self, block: &Block) -> CanAddResult
    {
        // Check duplicate
        let existing_block_or_none = self.blocks.get(&block.block_id);
        if existing_block_or_none.is_some()
        {
            let existing_block = existing_block_or_none.unwrap();
            if existing_block == block {
                return CanAddResult::Duplicate;
            } else {
                return CanAddResult::Invalid;
            }
        }

        // Check this is the next block in the chain
        if block.block_id == self.top + 1
        {
            let last = self.blocks.get(&(block.block_id - 1));
            if block.is_next_block(last.unwrap()).is_ok() {
                return CanAddResult::Yes;
            }
        }
        
        // Check this is the previus block in the chain
        if block.block_id == self.bottom - 1
        {
            let next = self.blocks.get(&(block.block_id + 1));
            if next.unwrap().is_next_block(block).is_ok() {
                return CanAddResult::Yes;
            }
        }
       
        // Check sub branches
        for (id, sub_branch) in &self.sub_branches 
        {
            match sub_branch.can_add(block)
            {
                CanAddResult::Invalid => 
                    {},

                CanAddResult::Duplicate => 
                    return CanAddResult::Duplicate,

                CanAddResult::Yes | CanAddResult::InSubBranch(_) => 
                    return CanAddResult::InSubBranch(*id),
            }
        }

        CanAddResult::Invalid
    }

    pub fn try_add(&mut self, block: &Block) -> CanAddResult
    {
        match self.can_add(block)
        {
            CanAddResult::Yes =>
            {
                self.top = std::cmp::max(self.top, block.block_id);
                self.bottom = std::cmp::min(self.bottom, block.block_id);
                self.blocks.insert(block.block_id, block.clone());
                CanAddResult::Yes
            },

            CanAddResult::InSubBranch(id) =>
            {
                let branch = &mut self.sub_branches.get_mut(&id);
                assert_eq!(branch.is_some(), true);
                branch.as_mut().unwrap().try_add(block)
            },

            err => err,
        }
    }

    pub fn next_missing_block(&self) -> Option<u64>
    {
        if self.bottom > 1 {
            Some( self.bottom - 1 )
        } else {
            None
        }
    }

    pub fn is_complete(&self) -> bool
    {
        self.bottom == 1
    }

    pub fn is_empty(&self) -> bool
    {
        self.top < self.bottom
    }

    fn get_merge_type(&self, other: &Branch) -> MergeType
    {
        // Extends the top
        if other.bottom == self.top + 1
        {
            let our_top = &self.blocks[&self.top];
            let other_bottom = &other.blocks[&other.bottom];
            if other_bottom.is_next_block(our_top).is_ok() {
                return MergeType::Extends;
            }
        }

        // Extends the bottom
        if other.top == self.bottom - 1
        {
            let other_top = &other.blocks[&other.top];
            let our_bottom = &self.blocks[&self.bottom];
            if our_bottom.is_next_block(other_top).is_ok() {
                return MergeType::Extends;
            }
        }

        // Is other a sub-chain of us
        if other.bottom > self.bottom && other.bottom < self.top 
        {
            let root = &self.blocks[&(other.bottom - 1)];
            let next = &other.blocks[&other.bottom];
            if next.is_next_block(root).is_ok() {
                return MergeType::OtherIsSubBranch;
            }
        }

        // Are we a sub-chain of other
        if self.bottom > other.bottom && self.bottom < other.top 
        {
            let root = &other.blocks[&(self.bottom - 1)];
            let next = &self.blocks[&self.bottom];
            if next.is_next_block(root).is_ok() {
                return MergeType::WeAreSubChain;
            }
        }

        // Check sub branches
        for (id, sub_branch) in &self.sub_branches
        {
            match sub_branch.get_merge_type(other)
            {
                MergeType::NoMerge => 
                    {},

                _ => 
                    return MergeType::MergeSubBranch(*id),
            }
        }

        MergeType::NoMerge
    }

    pub fn can_merge(&self, other: &Branch) -> bool
    {
        self.get_merge_type(other) != MergeType::NoMerge
    }

    fn add_sub_branch(&mut self, sub_branch: Branch)
    {
        let mut branch_id = rand::random::<i32>();
        while self.sub_branches.contains_key(&branch_id) {
            branch_id = rand::random::<i32>();
        }
        self.sub_branches.insert(branch_id, sub_branch);
    }

    pub fn merge(&mut self, mut other: Branch)
    {
        match self.get_merge_type(&other)
        {
            MergeType::Extends =>
            {
                for i in other.bottom..=other.top {
                    self.blocks.insert(i, other.blocks[&i].clone());
                }

                self.top = std::cmp::max(self.top, other.top);
                self.bottom = std::cmp::min(self.bottom, other.bottom);
            },

            MergeType::OtherIsSubBranch =>
            {
                self.add_sub_branch(other);
            },

            MergeType::WeAreSubChain =>
            {
                std::mem::swap(self, &mut other);
                self.add_sub_branch(other);
            },

            MergeType::MergeSubBranch(id) =>
            {
                let sub_branch = &mut self.sub_branches.get_mut(&id).unwrap();
                sub_branch.merge(other);
            },

            MergeType::NoMerge =>
            {
                panic!();
            },
        }
    }

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
        if !block.is_valid() {
            return false;
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

    pub fn remove_invalid_blocks(&mut self, logger: &mut Logger<impl Write>) -> Vec<Branch>
    {
        if !self.is_complete() {
            Vec::new()
        } else {
            self.remove_invalid_blocks_impl(None, logger)
        }
    }

    fn longest_sub_branch(&self) -> Option<&Branch>
    {
        let mut current_longest: Option<&Branch> = None;
        for (_, sub_branch) in &self.sub_branches 
        {
            let longest_sub = sub_branch.longest_sub_branch().unwrap_or(sub_branch);

            if longest_sub.top <= self.top {
                continue;
            }

            if current_longest.is_none() || longest_sub.top > current_longest.unwrap().top { 
                current_longest = Some( sub_branch );
            }
        }

        current_longest
    }

    pub fn top(&self) -> &Block
    {
        match self.longest_sub_branch()
        {
            Some(sub_branch) =>
                sub_branch.top(),

            None =>
                &self.blocks[&self.top],
        }
    }

    pub fn block(&self, block_id: u64) -> Option<&Block>
    {
        match self.longest_sub_branch()
        {
            Some(sub_branch) =>
            {
                if block_id >= sub_branch.bottom {
                    sub_branch.block(block_id)
                } else {
                    self.blocks.get(&block_id)
                }
            },

            None =>
                self.blocks.get(&block_id)
        }
    }

    pub fn walk(&self, on_block: &mut impl FnMut(&Block))
    {
        let len = self.top().block_id;
        for i in 1..=len 
        {
            let block = self
                .block(i)
                .expect(&format!("Has block {}", i));

            on_block(block);
        }
    }

}

impl fmt::Display for Branch
{

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
    {
        write!(f, "{} -> {} {:?}", self.bottom, self.top, 
           self.sub_branches.iter().map(|(_, x)| format!("{}", x)).collect::<Vec<_>>())
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::miner;
    use crate::block::HASH_LEN;
    
    fn create_blocks(count: u64, start_id: u64, start_hash: Option<[u8; HASH_LEN]>) -> Vec<Block>
    {
        let mut blocks = Vec::<Block>::new();
        let mut prev_hash = start_hash.unwrap_or([0u8; HASH_LEN]);

        for i in 0..count
        {
            let block = miner::mine_block(Block::new_debug(start_id + i, prev_hash));
            prev_hash = block.hash().expect("Hash worked");
            blocks.push(block);
        }

        blocks
    }

    #[test]
    fn test_sub_branches()
    {
        let chain = create_blocks(5, 1, None);
        let chain_branch_a = create_blocks(3, 4, Some( chain[2].hash().unwrap() ));
        let chain_branch_b = create_blocks(4, 4, Some( chain[2].hash().unwrap() ));
        let chain_branch_a_branch = create_blocks(3, 5, Some( chain_branch_a[0].hash().unwrap() ));
        
        let make_branch_from_chain = |chain: &Vec<Block>|
        {
            let mut branch = Branch::new(chain[0].clone());
            for i in 1..chain.len() {
                branch.try_add(&chain[i]);
            }

            return branch;
        };

        let mut branch_a = make_branch_from_chain(&chain);
        let branch_b = make_branch_from_chain(&chain_branch_a);
        let branch_c = make_branch_from_chain(&chain_branch_b);
        let branch_d = make_branch_from_chain(&chain_branch_a_branch);

        assert_eq!(branch_a.can_merge(&branch_b), true);
        branch_a.merge(branch_b);
        assert_eq!(branch_a.can_merge(&branch_c), true);
        branch_a.merge(branch_c);
        assert_eq!(branch_a.can_merge(&branch_d), true);
        branch_a.merge(branch_d);
    }

    #[test]
    fn test_merge()
    {
        let chain = create_blocks(5, 1, None);
        
        let mut branch_a = Branch::new(chain[0].clone());
        branch_a.try_add(&chain[1]);
        branch_a.try_add(&chain[2]);

        let mut branch_b = Branch::new(chain[3].clone());
        branch_b.try_add(&chain[4]);

        assert_eq!(branch_a.can_merge(&branch_b), true);
        assert_eq!(branch_b.can_merge(&branch_a), true);
    }

    fn test_sequencial_add(test_blocks: &Vec<Block>)
    {
        let mut branch = Branch::new(test_blocks[0].clone());
        for i in 1..test_blocks.len() {
            assert_eq!(branch.try_add(&test_blocks[i]), CanAddResult::Yes);
        }
    }

    fn test_unordered_add(test_blocks: &Vec<Block>)
    {
        let mut branch = Branch::new(test_blocks[2].clone());
        assert_eq!(branch.try_add(&test_blocks[3]), CanAddResult::Yes);
        assert_eq!(branch.try_add(&test_blocks[1]), CanAddResult::Yes);
        assert_eq!(branch.try_add(&test_blocks[0]), CanAddResult::Yes);
    }

    fn test_invalid_blocks(test_blocks_branch_a: &Vec<Block>, test_blocks_branch_b: &Vec<Block>)
    {
        let mut branch = Branch::new(test_blocks_branch_a[0].clone());
        assert_eq!(branch.try_add(&test_blocks_branch_a[1]), CanAddResult::Yes);
        assert_eq!(branch.try_add(&test_blocks_branch_b[2]), CanAddResult::Invalid);
        assert_eq!(branch.try_add(&test_blocks_branch_b[1]), CanAddResult::Invalid);
        assert_eq!(branch.try_add(&test_blocks_branch_a[0]), CanAddResult::Duplicate);
        assert_eq!(branch.try_add(&test_blocks_branch_a[3]), CanAddResult::Invalid);
    }

    #[test]
    fn test_branches() 
    {
        let test_blocks_branch_a = create_blocks(4, 1, None);
        let test_blocks_branch_b = create_blocks(4, 1, None);

        test_sequencial_add(&test_blocks_branch_a);
        test_unordered_add(&test_blocks_branch_a);
        test_invalid_blocks(&test_blocks_branch_a, &test_blocks_branch_b);
    }

}
