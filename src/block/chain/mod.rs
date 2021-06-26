mod branch;
use crate::block::Block;
use crate::logger::{Logger, LoggerLevel};
use branch::{Branch, TryAddResult};

use std::io::Write;

pub struct BlockChain
{
    branches: Vec<Branch>,
}

pub enum BlockChainAddResult
{
    Ok,
    MoreNeeded(u64),
    Duplicate,
}

impl BlockChain
{

    pub fn new(logger: &mut Logger<impl Write>) -> Self
    {
        logger.log(LoggerLevel::Info, "Create new chain");
        BlockChain
        {
            branches: Vec::new(),
        }
    }

    pub fn add(&mut self, block: &Block, logger: &mut Logger<impl Write>) -> BlockChainAddResult
    {
        let block_id = block.block_id;

        // Try to add the block to an existing branch
        for index in 0..self.branches.len()
        {
            let branch = &mut self.branches[index];
            match branch.try_add(&block)
            {
                TryAddResult::Success => 
                {
                    logger.log(LoggerLevel::Info, &format!("Added block {} to branch {}", block_id, index));
                    return match branch.next_missing_block()
                    {
                        Some(id) => BlockChainAddResult::MoreNeeded(id),
                        None => BlockChainAddResult::Ok,
                    };
                },

                TryAddResult::Duplicate =>
                {
                    logger.log(LoggerLevel::Info, &format!("Duplicate block {} in branch {}", block_id, index));
                    return BlockChainAddResult::Duplicate;
                },

                TryAddResult::Invalid => {},
            }
        }

        // If we couldn't create a new one
        logger.log(LoggerLevel::Info, &format!("Added block {} to new branch", block_id));
        let mut branch = Branch::new();
        assert_eq!(branch.try_add(&block), TryAddResult::Success);
        
        let result = match branch.next_missing_block()
        {
            Some(id) => BlockChainAddResult::MoreNeeded(id),
            None => BlockChainAddResult::Ok,
        };
        self.branches.push(branch);
        result
    }

    fn find_longest_complete_branch(&self) -> Option<&Branch>
    {
        let mut max_branch = None;
        let mut max_length = 0;

        for branch in &self.branches
        {
            let length_or_none = branch.length_if_complete();
            if length_or_none.is_none() {
                continue;
            }

            let length = length_or_none.unwrap();
            if length > max_length
            {
                max_length = length;
                max_branch = Some( branch );
            }
        }

        return max_branch;
    }

    pub fn top(&self) -> Option<&Block>
    {
        let lengest_branch = self.find_longest_complete_branch();
        if lengest_branch.is_some() {
            lengest_branch.unwrap().top()
        } else {
            None
        }
    }

    pub fn top_id(&self) -> u64
    {
        let top = self.top();
        if top.is_some() {
            top.unwrap().block_id
        } else {
            0
        }
    }

    pub fn block(&self, block_id: u64) -> Option<&Block>
    {
        let lengest_branch = self.find_longest_complete_branch();
        if lengest_branch.is_some() {
            lengest_branch.unwrap().block(block_id)
        } else {
            None
        }
    }

    pub fn walk_chain(&self, on_block: &mut impl FnMut(&Block))
    {
        let longest_branch_or_none = self.find_longest_complete_branch();
        if longest_branch_or_none.is_none() {
            return;
        }

        let longest_branch = longest_branch_or_none.unwrap();
        for i in 1..(longest_branch.len() + 1) 
        {
            let block = longest_branch
                .block(i)
                .expect(&format!("Has block {}", i));

            on_block(block);
        }
    }

    pub fn debug_log_chain<W: Write>(&self, logger: &mut Logger<W>)
    {
        for index in 0..self.branches.len()
        {
            let branch = &self.branches[index];
            logger.log(LoggerLevel::Info, &format!("{}: {}", index, branch));
        }
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::block::HASH_LEN;
    use crate::miner;

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

    #[test]
    fn test_block_chain()
    {
        let test_blocks_a = create_blocks(4);
        let test_blocks_b = create_blocks(5);
        let test_blocks_c = create_blocks(5);

        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::new(&mut logger);
        
        // Add one chain in a random order
        chain.add(&test_blocks_a[3], &mut logger);
        chain.add(&test_blocks_a[2], &mut logger);
        chain.add(&test_blocks_a[0], &mut logger);
        chain.add(&test_blocks_a[1], &mut logger);
        assert_eq!(chain.branches.len(), 1);
        assert_eq!(chain.branches[0].length_if_complete(), Some( 4 ));
        assert_eq!(chain.top().is_some(), true);
        assert_eq!(chain.top().unwrap().block_id, 4);

        // Add two random blocks from a different chain
        chain.add(&test_blocks_b[2], &mut logger);
        chain.add(&test_blocks_b[0], &mut logger);
        assert_eq!(chain.branches.len(), 2);
        assert_eq!(chain.branches[0].length_if_complete(), Some( 4 ));
        assert_eq!(chain.branches[1].length_if_complete(), None);

        // Add the last block to complete the second chain
        chain.add(&test_blocks_b[1], &mut logger);
        assert_eq!(chain.branches.len(), 2);
        assert_eq!(chain.branches[0].length_if_complete(), Some( 4 ));
        assert_eq!(chain.branches[1].length_if_complete(), Some( 3 ));
        assert_eq!(chain.top().expect("Has top").block_id, 4);

        // Add the rest of the second chain and a duplicate node from a third
        chain.add(&test_blocks_b[3], &mut logger);
        chain.add(&test_blocks_c[4], &mut logger);
        chain.add(&test_blocks_b[4], &mut logger);
        assert_eq!(chain.branches.len(), 3);
        assert_eq!(chain.branches[0].length_if_complete(), Some( 4 ));
        assert_eq!(chain.branches[1].length_if_complete(), Some( 5 ));
        assert_eq!(chain.branches[2].length_if_complete(), None);
        assert_eq!(chain.top().expect("Has top").block_id, 5);
    }

}
