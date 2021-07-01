use crate::block::Block;
use std::collections::HashMap;
use std::fmt;

pub struct Branch
{
    blocks: HashMap<u64, Block>,
    top: u64,
    bottom: u64,
}

#[derive(PartialEq, Debug)]
pub enum TryAddResult
{
    Success,
    Duplicate,
    Invalid,
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
            top: start_id,
            bottom: start_id,
        }
    }

    pub fn try_add(&mut self, block: &Block) -> TryAddResult
    {
        // Check duplicate
        let existing_block_or_none = self.blocks.get(&block.block_id);
        if existing_block_or_none.is_some()
        {
            let existing_block = existing_block_or_none.unwrap();
            if existing_block == block {
                return TryAddResult::Duplicate;
            } else {
                return TryAddResult::Invalid;
            }
        }

        // Check this is the next block in the chain
        if block.block_id == self.top + 1
        {
            let last = self.blocks.get(&(block.block_id - 1));
            if block.is_next_block(last.unwrap()).is_ok() 
            {
                self.top = block.block_id;
                self.blocks.insert(block.block_id, block.clone());
                return TryAddResult::Success;
            }
        }
        
        // Check this is the previus block in the chain
        if block.block_id == self.bottom - 1
        {
            let next = self.blocks.get(&(block.block_id + 1));
            if next.unwrap().is_next_block(block).is_ok() 
            {
                self.bottom = block.block_id;
                self.blocks.insert(block.block_id, block.clone());
                return TryAddResult::Success;
            }
        }

        TryAddResult::Invalid
    }

    pub fn next_missing_block(&self) -> Option<u64>
    {
        if self.bottom > 1 {
            Some( self.bottom - 1 )
        } else {
            None
        }
    }

    pub fn length_if_complete(&self) -> Option<u64>
    {
        // Check the chain starts at 1
        let mut last = self.blocks.get(&1);
        if last.is_none() {
            return None;
        }

        // Check we have blocks from 1 to length without gaps
        let mut length = 1;
        loop
        {
            let next = self.blocks.get(&(length + 1));
            if next.is_none() {
                break
            }

            assert_eq!(next.unwrap().is_next_block(last.unwrap()).is_ok(), true);
            last = next;
            length += 1;
        }

        if length == self.blocks.len() as u64 {
            Some( length )
        } else {
            None
        }
    }

    pub fn can_merge(&self, other: &Branch) -> bool
    {
        // Extends the top
        if other.bottom == self.top + 1
        {
            let our_top = &self.blocks[&self.top];
            let other_bottom = &other.blocks[&other.bottom];
            return other_bottom.is_next_block(our_top).is_ok();
        }

        // Extends the bottom
        if other.top == self.bottom - 1
        {
            let other_top = &other.blocks[&other.top];
            let our_bottom = &self.blocks[&self.bottom];
            return our_bottom.is_next_block(other_top).is_ok();
        }

        false
    }

    pub fn merge(&mut self, other: Branch)
    {
        assert_eq!(self.can_merge(&other), true);

        for i in other.bottom..=other.top {
            self.blocks.insert(i, other.blocks[&i].clone());
        }

        self.top = std::cmp::max(self.top, other.top);
        self.bottom = std::cmp::min(self.bottom, other.bottom);
    }

    pub fn len(&self) -> u64
    {
        self
            .length_if_complete()
            .expect("Is complete")
    }

    pub fn top(&self) -> &Block
    {
        &self.blocks[&self.top]
    }

    pub fn block(&self, block_id: u64) -> Option<&Block>
    {
        self.blocks.get(&block_id)
    }

}

impl fmt::Display for Branch
{

    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result 
    {
        write!(f, "{} -> {}", self.bottom, self.top)
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::miner;
    use crate::block::HASH_LEN;
    
    fn create_blocks(count: u64, start_hash: Option<[u8; HASH_LEN]>) -> Vec<Block>
    {
        let mut blocks = Vec::<Block>::new();
        let mut prev_hash = start_hash.unwrap_or([0u8; HASH_LEN]);

        for i in 1..(count + 1)
        {
            let block = miner::mine_block(Block::new_debug(i, prev_hash));
            prev_hash = block.hash().expect("Hash worked");
            blocks.push(block);
        }

        blocks
    }

    #[test]
    fn test_merge()
    {
        let chain = create_blocks(5, None);
        
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
            assert_eq!(branch.try_add(&test_blocks[i]), TryAddResult::Success);
        }
    }

    fn test_unordered_add(test_blocks: &Vec<Block>)
    {
        let mut branch = Branch::new(test_blocks[2].clone());
        assert_eq!(branch.try_add(&test_blocks[3]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks[1]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks[0]), TryAddResult::Success);
    }

    fn test_invalid_blocks(test_blocks_branch_a: &Vec<Block>, test_blocks_branch_b: &Vec<Block>)
    {
        let mut branch = Branch::new(test_blocks_branch_a[0].clone());
        assert_eq!(branch.try_add(&test_blocks_branch_a[1]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks_branch_b[2]), TryAddResult::Invalid);
        assert_eq!(branch.try_add(&test_blocks_branch_b[1]), TryAddResult::Invalid);
        assert_eq!(branch.try_add(&test_blocks_branch_a[0]), TryAddResult::Duplicate);
        assert_eq!(branch.try_add(&test_blocks_branch_a[3]), TryAddResult::Invalid);
    }

    #[test]
    fn test_branches() 
    {
        let test_blocks_branch_a = create_blocks(4, None);
        let test_blocks_branch_b = create_blocks(4, None);

        test_sequencial_add(&test_blocks_branch_a);
        test_unordered_add(&test_blocks_branch_a);
        test_invalid_blocks(&test_blocks_branch_a, &test_blocks_branch_b);
    }

}
