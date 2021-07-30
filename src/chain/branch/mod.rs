pub mod prune;
pub mod add;
pub mod merge;
pub mod validate;
use crate::block::Block;

use std::collections::HashMap;
use std::fmt;

pub struct Branch
{
    blocks: HashMap<u64, Block>,
    sub_branches: HashMap<i32, Branch>,
    top: u64,
    bottom: u64,
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
    use super::add::{CanAddResult, BranchAdd};
    use super::merge::BranchMerge;
    use crate::miner;
    use crate::block::HASH_LEN;
    
    pub fn create_blocks(count: u64, start_id: u64, start_hash: Option<[u8; HASH_LEN]>) -> Vec<Block>
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

