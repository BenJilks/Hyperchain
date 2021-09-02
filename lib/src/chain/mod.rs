pub mod branch;
mod transaction_queue;
mod storage;
mod transactions;
mod metadata;
use storage::Storage;
use metadata::BlockMetadata;
use crate::block::Block;
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
    metadata: Storage<BlockMetadata>,
    blocks: Storage<Block>,
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
            metadata: Storage::new(&path.join("metadata"))?,
            blocks: Storage::new(path)?,
            transaction_queue: VecDeque::new(),
        })
    }

    pub fn take_sample_at(&mut self, block_id: u64) -> (Option<Block>, Option<Block>)
    {
        let end = self.block(block_id);
        if end.is_none() || end.as_ref().unwrap().block_id < BLOCK_SAMPLE_SIZE {
            return (None, end);
        }

        let start = self.block(end.as_ref().unwrap().block_id - BLOCK_SAMPLE_SIZE);
        (start, end)
    }

    pub fn take_sample(&mut self) -> (Option<Block>, Option<Block>)
    {
        match self.top()
        {
            Some(top) => self.take_sample_at(top.block_id),
            None => (None, None),
        }
    }

    pub fn add<W>(&mut self, block: &Block, logger: &mut Logger<W>) 
            -> Result<BlockChainAddResult, Box<dyn Error>>
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

        match self.validate_branch(&[block.clone()])?
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

        let metadata = self.metadata_for_block(&block);
        self.metadata.store(block.block_id, metadata);
        self.blocks.store(block.block_id, block.clone());
        self.remove_from_transaction_queue(block);
        Ok(BlockChainAddResult::Ok)
    }

    pub fn walk<F>(&mut self, on_block: &mut F)
        where F: FnMut(&Block)
    {
        for block_id in 0..self.blocks.next_top() {
            on_block(&self.block(block_id).unwrap());
        }
    }

    pub fn block(&mut self, block_id: u64) -> Option<Block>
    {
        self.blocks.get(block_id)
    }

    pub fn top(&mut self) -> Option<Block>
    {
        if self.blocks.next_top() == 0 {
            None
        } else {
            self.blocks.get(self.blocks.next_top() - 1)
        }
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
        
        let block_a = miner::mine_block(Block::new(&mut chain_a, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_a, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_a, &mut logger).unwrap(), BlockChainAddResult::Ok);

        let block_b = miner::mine_block(Block::new(&mut chain_a, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_b, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

        let block_c_a = miner::mine_block(Block::new(&mut chain_a, &wallet).unwrap());
        let block_c_b = miner::mine_block(Block::new(&mut chain_b, &wallet).unwrap());
        assert_eq!(chain_a.add(&block_c_a, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_b.add(&block_c_b, &mut logger).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain_a.add(&block_b, &mut logger).unwrap(), BlockChainAddResult::Duplicate);

        let block_d_b = miner::mine_block(Block::new(&mut chain_b, &wallet).unwrap());
        assert_eq!(chain_b.add(&block_d_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

        let block_e_b = miner::mine_block(Block::new(&mut chain_b, &wallet).unwrap());
        assert_eq!(chain_b.add(&block_e_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

        assert_eq!(chain_a.add(&block_e_b, &mut logger).unwrap(), BlockChainAddResult::MoreNeeded);
        assert_ne!(chain_a.add(&block_d_b, &mut logger).unwrap(), BlockChainAddResult::Ok);

        let mut branch = Vec::<Block>::new();
        branch.push(block_c_b);
        assert_ne!(chain_a.can_merge_branch(&branch).unwrap(), BlockChainCanMergeResult::Ok);

        branch.push(block_d_b);
        branch.push(block_e_b);
        assert_eq!(chain_a.can_merge_branch(&branch).unwrap(), BlockChainCanMergeResult::Ok);

        chain_a.merge_branch(branch, &mut logger);
        assert_eq!(chain_a.top().unwrap().block_id, 4);
   }

}
