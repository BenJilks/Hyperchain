use crate::block::Block;
use std::collections::HashMap;
use std::fmt;

pub struct Branch
{
    blocks: HashMap<u64, Block>,
}

#[derive(PartialEq, Debug)]
pub enum TryAddResult
{
    Success,
    Invalid,
}

impl Branch
{

    pub fn new() -> Self
    {
        Branch
        {
            blocks: HashMap::new(),
        }
    }

    pub fn try_add(&mut self, block: &Block) -> TryAddResult
    {
        // Check duplicate
        if self.blocks.contains_key(&block.block_id) {
            return TryAddResult::Invalid;
        }

        // Check this is the next block in the chain
        let last = self.blocks.get(&(block.block_id - 1));
        if last.is_some() && block.is_next_block(last.unwrap()).is_err() {
            return TryAddResult::Invalid;
        }

        // Check this is the previus block
        let next = self.blocks.get(&(block.block_id + 1));
        if next.is_some() && next.unwrap().is_next_block(block).is_err() 
        {
            // TODO: Create a new branch here
            panic!();
        }

        self.blocks.insert(block.block_id, block.clone());
        TryAddResult::Success
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

    pub fn len(&self) -> u64
    {
        self
            .length_if_complete()
            .expect("Is complete")
    }

    pub fn top(&self) -> Option<&Block>
    {
        let top_or_none = self.blocks.iter().max_by(|a, b| a.0.cmp(&b.0));
        if top_or_none.is_none() {
            None
        } else {
            Some( top_or_none.unwrap().1 )
        }
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
        write!(f, "{}", self.blocks.len())
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::miner;
    use crate::block::HASH_LEN;
    
    fn create_blocks(count: u64) -> Vec<Block>
    {
        let mut blocks = Vec::<Block>::new();
        let mut prev_hash = [0u8; HASH_LEN];

        for i in 1..(count + 1)
        {
            let block = miner::mine_block(Block::new_debug(i, prev_hash));
            prev_hash = block.hash().expect("Hash worked");
            blocks.push(block);
        }

        blocks
    }

    fn test_sequencial_add(test_blocks: &Vec<Block>)
    {
        let mut branch = Branch::new();
        for block in test_blocks {
            assert_eq!(branch.try_add(block), TryAddResult::Success);
        }
    }

    fn test_unordered_add(test_blocks: &Vec<Block>)
    {
        let mut branch = Branch::new();
        assert_eq!(branch.try_add(&test_blocks[3]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks[2]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks[0]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks[1]), TryAddResult::Success);
    }

    fn test_invalid_blocks(test_blocks_branch_a: &Vec<Block>, test_blocks_branch_b: &Vec<Block>)
    {
        let mut branch = Branch::new();
        assert_eq!(branch.try_add(&test_blocks_branch_a[0]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks_branch_a[1]), TryAddResult::Success);
        assert_eq!(branch.try_add(&test_blocks_branch_b[2]), TryAddResult::Invalid);
        assert_eq!(branch.try_add(&test_blocks_branch_b[1]), TryAddResult::Invalid);
        assert_eq!(branch.try_add(&test_blocks_branch_a[0]), TryAddResult::Invalid);
    }

    #[test]
    fn test_branches() 
    {
        let test_blocks_branch_a = create_blocks(4);
        let test_blocks_branch_b = create_blocks(4);

        test_sequencial_add(&test_blocks_branch_a);
        test_unordered_add(&test_blocks_branch_a);
        test_invalid_blocks(&test_blocks_branch_a, &test_blocks_branch_b);
    }

}
