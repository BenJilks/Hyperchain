pub mod branch;
mod transaction_queue;
mod storage;
use storage::Storage;
use crate::block::{Block, Hash};
use crate::block::validate::BlockValidationResult;
use crate::block::target::BLOCK_SAMPLE_SIZE;
use crate::logger::{Logger, LoggerLevel};
use crate::transaction::Transaction;

use std::collections::VecDeque;
use std::io::Write;
use std::error::Error;
use std::path::PathBuf;

pub struct BlockChain
{
    blocks: Storage,
    transaction_queue: VecDeque<Transaction>,
}

#[derive(Debug, PartialEq)]
pub enum BlockChainAddResult
{
    Ok,
    MoreNeeded,
    Duplicate,
    Invalid(BlockValidationResult),
}

impl BlockChain
{

    pub fn open(path: &PathBuf, logger: &mut Logger<impl Write>) 
        -> Result<Self, Box<dyn Error>>
    {
        logger.log(LoggerLevel::Info, &format!("Open chain in {:?}", path));
        Ok(BlockChain
        {
            blocks: Storage::new(path)?,
            transaction_queue: VecDeque::new(),
        })
    }

    pub fn take_sample_at(&self, block_id: u64) -> (Option<Block>, Option<Block>)
    {
        let end = self.block(block_id);
        if end.is_none() || end.as_ref().unwrap().block_id < BLOCK_SAMPLE_SIZE {
            return (None, end);
        }

        let start = self.block(end.as_ref().unwrap().block_id - BLOCK_SAMPLE_SIZE);
        (start, end)
    }

    pub fn take_sample(&self) -> (Option<Block>, Option<Block>)
    {
        match self.top()
        {
            Some(top) => self.take_sample_at(top.block_id),
            None => (None, None),
        }
    }

    fn remove_from_transaction_queue(&mut self, block: &Block)
    {
        for transaction in &block.transactions
        {
            let index = self.transaction_queue.iter().position(|x| x == transaction);
            if index.is_some() {
                self.transaction_queue.remove(index.unwrap());
            }
        }
    }

    pub fn add<W>(&mut self, block: &Block, logger: &mut Logger<W>) -> Result<BlockChainAddResult, Box<dyn Error>>
        where W: Write
    {
        if block.block_id < self.blocks.next_top() as u64
        {
            let original = self.block(block.block_id).unwrap();
            if block == &original {
                return Ok(BlockChainAddResult::Duplicate);
            } else {
                return Ok(BlockChainAddResult::Invalid(BlockValidationResult::NotNextBlock));
            }
        }

        if block.block_id > self.blocks.next_top() as u64 {
            return Ok(BlockChainAddResult::MoreNeeded);
        }

        match block.validate(self)?
        {
            BlockValidationResult::Ok => {},
            BlockValidationResult::Balance(address) =>
            {
                logger.log(LoggerLevel::Warning, 
                    &format!("Got invalid block, as {} has insufficient balance",
                        base_62::encode(&address)));

                // NOTE: Purge any pending transactions coming from this address
                self.transaction_queue
                    .iter()
                    .take_while(|x| x.get_from_address() == address)
                    .count();

                return Ok(BlockChainAddResult::Invalid(BlockValidationResult::Balance(address)));
            },
            result => return Ok(BlockChainAddResult::Invalid(result)),
        }

        self.remove_from_transaction_queue(block);
        self.blocks.store(block.clone());
        Ok(BlockChainAddResult::Ok)
    }

    fn take_sample_of_branch_at(&self, branch: &[Block], block_id: u64) 
        -> (Option<Block>, Option<Block>)
    {
        assert_eq!(branch.is_empty(), false);

        if block_id < BLOCK_SAMPLE_SIZE {
            return (None, None);
        }

        let branch_start = branch.first().unwrap();
        let block_at = |block_id: u64| -> Option<Block>
        {
            if block_id >= branch_start.block_id 
            {
                match branch.get((block_id - branch_start.block_id) as usize)
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

    pub fn walk<F>(&self, on_block: &mut F)
        where F: FnMut(&Block)
    {
        for block_id in 0..self.blocks.next_top() {
            on_block(&self.block(block_id).unwrap());
        }
    }

    pub fn block(&self, block_id: u64) -> Option<Block>
    {
        self.blocks.get(block_id)
    }

    pub fn top(&self) -> Option<Block>
    {
        if self.blocks.next_top() == 0 {
            None
        } else {
            self.blocks.get(self.blocks.next_top() - 1)
        }
    }

    pub fn find_transaction_in_chain(&self, transaction_id: &Hash) 
        -> Option<(Transaction, Block)>
    {
        for block_id in 0..self.blocks.next_top() 
        {
            let block = self.block(block_id).unwrap();
            for transaction in &block.transactions
            {
                match transaction.header.hash()
                {
                    Ok(hash) =>
                    {
                        if hash == transaction_id {
                            return Some((transaction.clone(), block));
                        }
                    },
                    Err(_) => {},
                }
            }
        }

        None
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::branch::BlockChainCanMergeResult;
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::miner;

    use std::path::PathBuf;

    impl BlockChain
    {
        pub fn open_temp(logger: &mut Logger<impl Write>) -> Self
        {
            let path = std::env::temp_dir().join(rand::random::<u32>().to_string());
            Self::open(&path, logger).unwrap()
        }
    }

    impl Drop for BlockChain
    {
        fn drop(&mut self)
        {
            let _ = std::fs::remove_dir_all(self.blocks.path());
        }
    }

    #[test]
    fn test_block_chain()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain_a = BlockChain::open_temp(&mut logger);
        let mut chain_b = BlockChain::open_temp(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        
        let block_a = miner::mine_block(Block::new(&chain_a, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_a, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_a, &mut logger).unwrap(), BlockChainAddResult::Ok);

        let block_b = miner::mine_block(Block::new(&chain_a, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_b, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

        let block_c_a = miner::mine_block(Block::new(&chain_a, &wallet).unwrap());
        let block_c_b = miner::mine_block(Block::new(&chain_b, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_c_a, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_c_b, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_a.add(&block_b, &mut logger).unwrap(), BlockChainAddResult::Duplicate);

        let block_d_b = miner::mine_block(Block::new(&chain_b, &wallet).unwrap());
        assert_eq!(chain_b.add(&block_d_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

        let block_e_b = miner::mine_block(Block::new(&chain_b, &wallet).unwrap());
        assert_eq!(chain_b.add(&block_e_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

        assert_eq!(chain_a.add(&block_e_b, &mut logger).unwrap(), BlockChainAddResult::MoreNeeded);
        assert_ne!(chain_a.add(&block_d_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

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

