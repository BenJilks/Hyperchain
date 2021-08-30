use crate::block::Block;
use crate::block::validate::{BlockValidate, BlockValidationResult};
use crate::block::target::BLOCK_SAMPLE_SIZE;
use crate::logger::{Logger, LoggerLevel};

use std::io::Write;
use std::error::Error;

pub struct BlockChain
{
    blocks: Vec<Block>,
}

#[derive(Debug, PartialEq)]
pub enum BlockChainAddResult
{
    Ok,
    MoreNeeded,
    Duplicate,
    Invalid(BlockValidationResult),
}

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

    pub fn new(logger: &mut Logger<impl Write>) -> Self
    {
        logger.log(LoggerLevel::Info, "Create new chain");
        BlockChain
        {
            blocks: Vec::new(),
        }
    }

    fn take_sample_at(&self, block_id: u64) -> (Option<&Block>, Option<&Block>)
    {
        let end = self.block(block_id);
        if end.is_none() || end.unwrap().block_id < BLOCK_SAMPLE_SIZE {
            return (None, None);
        }

        let start = self.block(end.unwrap().block_id - BLOCK_SAMPLE_SIZE);
        (start, end)
    }

    pub fn take_sample(&self) -> (Option<&Block>, Option<&Block>)
    {
        match self.top()
        {
            Some(top) => self.take_sample_at(top.block_id),
            None => (None, None),
        }
    }

    pub fn add(&mut self, block: &Block) -> Result<BlockChainAddResult, Box<dyn Error>>
    {
        if block.block_id < self.blocks.len() as u64 
        {
            let original = self.block(block.block_id).unwrap();
            if block == original {
                return Ok(BlockChainAddResult::Duplicate);
            } else {
                return Ok(BlockChainAddResult::Invalid(BlockValidationResult::NotNextBlock));
            }
        }

        if block.block_id > self.blocks.len() as u64 {
            return Ok(BlockChainAddResult::MoreNeeded);
        }

        if self.top().is_some()
        {
            match block.is_next_block(self.top().unwrap())?
            {
                BlockValidationResult::Ok => {},
                result => return Ok(BlockChainAddResult::Invalid(result)),
            }
        }

        let (sample_start, sample_end) = self.take_sample();
        match block.is_valid(sample_start, sample_end)?
        {
            BlockValidationResult::Ok => {},
            result => return Ok(BlockChainAddResult::Invalid(result)),
        }

        self.blocks.push(block.clone());
        Ok(BlockChainAddResult::Ok)
    }

    fn take_sample_of_branch_at<'a>(&'a self, branch: &'a [Block], block_id: u64) -> (Option<&'a Block>, Option<&'a Block>)
    {
        assert_eq!(branch.is_empty(), false);

        if block_id < BLOCK_SAMPLE_SIZE {
            return (None, None);
        }

        let branch_start = branch.first().unwrap();
        let block_at = |block_id: u64|
        {
            if block_id >= branch_start.block_id {
                branch.get((block_id - branch_start.block_id) as usize)
            } else {
                self.block(block_id)
            }
        };

        let sample_start = block_at(block_id - BLOCK_SAMPLE_SIZE);
        let sample_end = block_at(block_id);
        (sample_start, sample_end)
    }

    pub fn can_merge_branch(&self, branch: &[Block]) 
        -> Result<BlockChainCanMergeResult, Box<dyn Error>>
    {
        if branch.is_empty() {
            return Ok(BlockChainCanMergeResult::Empty);
        }

        // Can't attach to chain
        let bottom = branch.first().unwrap();
        if bottom.block_id > self.blocks.len() as u64 {
            return Ok(BlockChainCanMergeResult::Above);
        }

        // Not longer then the current branch
        let top = branch.last().unwrap();
        if top.block_id < self.blocks.len() as u64 {
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

                let (sample_start, sample_end) = self.take_sample_of_branch_at(branch, last_block.block_id);
                match block.is_next_block(last_block)?
                {
                    BlockValidationResult::Ok => {},
                    result => return Ok(BlockChainCanMergeResult::Invalid(result)),
                }
                match block.is_valid(sample_start, sample_end)?
                {
                    BlockValidationResult::Ok => {},
                    result => return Ok(BlockChainCanMergeResult::Invalid(result)),
                }
            }

            last_block_or_none = Some( block );
        }
 
        Ok(BlockChainCanMergeResult::Ok)
    }

    pub fn merge_branch(&mut self, branch: Vec<Block>)
    {
        assert_eq!(self.can_merge_branch(&branch).unwrap(), BlockChainCanMergeResult::Ok);
        for block in branch
        {
            let block_id = block.block_id as usize;
            if block_id < self.blocks.len() {
                self.blocks[block_id] = block;
            } else {
                self.blocks.push(block);
            }
        }
    }

    pub fn walk<F>(&self, on_block: &mut F)
        where F: FnMut(&Block)
    {
        for block in &self.blocks {
            on_block(block);
        }
    }

    pub fn block(&self, block_id: u64) -> Option<&Block>
    {
        self.blocks.get(block_id as usize)
    }

    pub fn top(&self) -> Option<&Block>
    {
        self.blocks.last()
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::miner;

    use std::path::PathBuf;

    #[test]
    fn test_block_chain()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain_a = BlockChain::new(&mut logger);
        let mut chain_b = BlockChain::new(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        
        let block_a = miner::mine_block(Block::new(&chain_a, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_a).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_a).unwrap(), BlockChainAddResult::Ok);

        let block_b = miner::mine_block(Block::new(&chain_a, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_b).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_b).unwrap(), BlockChainAddResult::Ok);

        let block_c_a = miner::mine_block(Block::new(&chain_a, &wallet).unwrap());
        let block_c_b = miner::mine_block(Block::new(&chain_b, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_c_a).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_c_b).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_a.add(&block_b).unwrap(), BlockChainAddResult::Duplicate);

        let block_d_b = miner::mine_block(Block::new(&chain_b, &wallet).unwrap());
        assert_eq!(chain_b.add(&block_d_b).unwrap(), BlockChainAddResult::Ok);

        let block_e_b = miner::mine_block(Block::new(&chain_b, &wallet).unwrap());
        assert_eq!(chain_b.add(&block_e_b).unwrap(), BlockChainAddResult::Ok);

        assert_eq!(chain_a.add(&block_e_b).unwrap(), BlockChainAddResult::MoreNeeded);
        assert_ne!(chain_a.add(&block_d_b).unwrap(), BlockChainAddResult::Ok);

        let mut branch = Vec::<Block>::new();
        branch.push(block_c_b);
        assert_ne!(chain_a.can_merge_branch(&branch).unwrap(), BlockChainCanMergeResult::Ok);

        branch.push(block_d_b);
        branch.push(block_e_b);
        assert_eq!(chain_a.can_merge_branch(&branch).unwrap(), BlockChainCanMergeResult::Ok);

        chain_a.merge_branch(branch);
        assert_eq!(chain_a.top().unwrap().block_id, 4);
   }

}
