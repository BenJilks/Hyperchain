mod branch;
use crate::block::Block;
use crate::logger::{Logger, LoggerLevel};
use branch::{Branch, CanAddResult};

use std::collections::HashMap;
use std::io::Write;
use rand;

pub struct BlockChain
{
    branches: HashMap<i32, Branch>,
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
            branches: HashMap::new(),
        }
    }

    fn find_branch_id_and_add(&mut self, block: &Block, logger: &mut Logger<impl Write>)
        -> Result<i32, BlockChainAddResult>
    {
        // Try to add the block to an existing branch
        let mut valid_branch_id = None;
        for (id, branch) in &mut self.branches
        {
            match branch.can_add(&block)
            {
                CanAddResult::Yes | CanAddResult::InSubBranch(_) => 
                    valid_branch_id = Some( id ),

                CanAddResult::Duplicate =>
                    return Err( BlockChainAddResult::Duplicate ),

                CanAddResult::Invalid => 
                    {},
            }
        }

        if valid_branch_id.is_some() 
        {
            let branch_id = *valid_branch_id.unwrap();
            let branch = &mut self.branches.get_mut(&branch_id).unwrap();
            assert_eq!(branch.try_add(block), CanAddResult::Yes);

            logger.log(LoggerLevel::Info, 
               &format!("Added block {} to branch {}", block.block_id, branch_id));
            return Ok( branch_id );
        }

        // If we couldn't create a new one
        let branch = Branch::new(block.clone());

        // Generate unique, branch id
        let mut branch_id = rand::random::<i32>();
        while self.branches.contains_key(&branch_id) {
            branch_id = rand::random::<i32>();
        }

        self.branches.insert(branch_id, branch);
        logger.log(LoggerLevel::Info, 
            &format!("Added block {} to new branch {}", block.block_id, branch_id));

        return Ok ( branch_id );
    }

    fn check_merges_for_branch_id(&mut self, branch_id: i32, logger: &mut Logger<impl Write>)
    {
        loop
        {
            let mut did_merge = false;
            let keys = self.branches.keys().map(|x| *x).collect::<Vec<i32>>();
            for id in keys
            {
                if id == branch_id {
                    continue;
                }

                if self.branches[&branch_id].can_merge(&self.branches[&id])
                {
                    let other = self.branches.remove(&id).unwrap();
                    let branch = self.branches.get_mut(&branch_id).unwrap();
                    branch.merge(other);
                    logger.log(LoggerLevel::Info, 
                        &format!("Merged {} -> {}", branch_id, id));

                    did_merge = true;
                    break;
                }
            }

            if !did_merge {
                break;
            }
        }
    }

    pub fn add(&mut self, block: &Block, logger: &mut Logger<impl Write>) -> BlockChainAddResult
    {
        match self.find_branch_id_and_add(block, logger)
        {
            Ok(branch_id) =>
            {
                self.check_merges_for_branch_id(branch_id, logger);
                match self.branches[&branch_id].next_missing_block()
                {
                    Some(id) => BlockChainAddResult::MoreNeeded(id),
                    None => BlockChainAddResult::Ok,
                }
            },

            Err(err) => err,
        }
    }

    fn find_longest_complete_branch(&self) -> Option<&Branch>
    {
        let mut max_branch = None;
        let mut max_length = 0;

        for (_, branch) in &self.branches
        {
            if !branch.is_complete() {
                continue;
            }

            let length = branch.top().block_id;
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
            Some( lengest_branch.unwrap().top() )
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
        let longest_branch = self.find_longest_complete_branch();
        if longest_branch.is_some() {
            longest_branch.unwrap().block(block_id)
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
        let lenght = longest_branch.top().block_id;
        for i in 1..=lenght 
        {
            let block = longest_branch
                .block(i)
                .expect(&format!("Has block {}", i));

            on_block(block);
        }
    }

    pub fn debug_log_chain<W: Write>(&self, logger: &mut Logger<W>)
    {
        for id in self.branches.keys()
        {
            let branch = &self.branches[id];
            logger.log(LoggerLevel::Info, &format!("{}: {}", id, branch));
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

    fn chain_has_branches_of_lengths(chain: &BlockChain, lengths: &[Option<u64>])
    {
        let mut lengths_left = lengths.to_vec();
        for branch in chain.branches.values()
        {
            let length = 
                if branch.is_complete() { 
                    Some( branch.top().block_id ) 
                } else { 
                    None 
                };

            let index_or_none = lengths_left.iter().position(|x| x == &length);
            match index_or_none
            {
                Some(index) =>
                    { lengths_left.remove(index); },

                None => 
                    panic!("Branch or length {:?} should not be in chain", length),
            }
        }
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
        chain_has_branches_of_lengths(&chain, &[Some(4)]);
        assert_eq!(chain.top().is_some(), true);
        assert_eq!(chain.top().unwrap().block_id, 4);

        // Add two random blocks from a different chain
        chain.add(&test_blocks_b[2], &mut logger);
        chain.add(&test_blocks_b[0], &mut logger);
        chain_has_branches_of_lengths(&chain, &[Some(4), Some(1), None]);

        // Add the last block to complete the second chain
        chain.add(&test_blocks_b[1], &mut logger);
        chain_has_branches_of_lengths(&chain, &[Some(4), Some(3)]);
        assert_eq!(chain.top().expect("Has top").block_id, 4);

        // Add the rest of the second chain and a duplicate node from a third
        chain.add(&test_blocks_b[3], &mut logger);
        chain.add(&test_blocks_c[4], &mut logger);
        chain.add(&test_blocks_b[4], &mut logger);
        chain_has_branches_of_lengths(&chain, &[Some(4), Some(5), None]);
        assert_eq!(chain.top().expect("Has top").block_id, 5);
    }

}

