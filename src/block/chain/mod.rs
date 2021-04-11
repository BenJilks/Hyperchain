mod chunk;
mod block_storage;
mod main_branch;
mod sub_branch;
mod block_buffer;
use super::Block;
use crate::error::Error;
use crate::wallet::{Wallet, WalletStatus};
use crate::logger::{Logger, LoggerLevel};
use block_storage::BlockStorage;
use main_branch::MainBranch;
use sub_branch::SubBranch;
use block_buffer::BlockBuffer;

use std::fs;
use std::path::PathBuf;
use std::io::Write;

pub struct BlockChain
{
    path: PathBuf,
    main_chain: MainBranch,
    sub_chains: Vec<SubBranch>,
    buffer: BlockBuffer,
}

impl BlockChain
{

    pub fn new<W: Write>(path: PathBuf, logger: &mut Logger<W>) -> Self
    {
        fs::create_dir_all(&path).unwrap();
        let chain = Self
        {
            path: path.clone(),
            main_chain: MainBranch::new(path.join("main").clone()),
            sub_chains: SubBranch::load_sub_branches(&path.join("sub_branches")),
            buffer: BlockBuffer::new(),
        };

        logger.log(LoggerLevel::Info, &format!("Open blockchain '{:?}', with top {}", 
            path, chain.main_chain.top_id()));
        chain
    }

    fn is_valid_next_block(&self, block: &Block) -> Result<(), Error>
    {
        if block.block_id >= 1
        {
            if block.block_id != self.main_chain.top_id() + 1 {
                return Err(Error::NotNextBlock);
            }

            if !block.validate_pow() {
                return Err(Error::InvalidPOW);
            }
        }

        block.validate(self)
    }

    fn add_block_to_sub_chain<W: Write>(&mut self, block: Block, logger: &mut Logger<W>) -> SubBranch
    {
        for sub_chain in &mut self.sub_chains
        {
            let existing_block = sub_chain.block(block.block_id);
            if existing_block.is_some() && existing_block.unwrap() == block 
            {
                logger.log(LoggerLevel::Info, &format!("Ignoring duplicate block {}", block.block_id));
                return sub_chain.clone();
            }

            if sub_chain.add_block(&block) {
                return sub_chain.clone();
            }
        }

        logger.log(LoggerLevel::Info, &format!("Creating new sub chain for {}", block.block_id));

        let new_id = SubBranch::generate_sub_branch_id(&self.path);
        let mut new_sub_chain = SubBranch::from(self.path.join("sub_branches").join(new_id));
        assert_eq!(new_sub_chain.add_block(&block), true);

        self.sub_chains.push(new_sub_chain.clone());
        return new_sub_chain;
    }

    fn delete_sub_chain(&mut self, sub_chain: &SubBranch)
    {
        std::fs::remove_dir_all(&sub_chain.path).unwrap();
        let index = self.sub_chains.iter().position(|x| x == sub_chain).unwrap();
        self.sub_chains.remove(index);
    }

    fn check_sub_chains_to_combine<W: Write>(a: &mut SubBranch, b: &SubBranch, logger: &mut Logger<W>) -> bool
    {
        if SubBranch::can_combine(a, b) 
        {
            logger.log(LoggerLevel::Info, &format!("Combine {} -> {} with {} -> {}", 
                a.bottom.unwrap(), a.top.unwrap(),
                b.bottom.unwrap(), b.top.unwrap()));

            a.combine_with(&b);
            return true;
        }

        return false;
    }

    fn find_pair<'a, T, F>(vec: &'a mut Vec<T>, callback: &mut F) -> Option<(&'a mut T, T)>
        where 
            T: PartialEq + Clone, 
            F: FnMut(&mut T, &T) -> bool
    {
        let other = vec.clone();
        for a in vec.iter_mut()
        {
            for b in &other
            {
                if a == b {
                    continue;
                }

                if callback(a, b) {
                    return Some((a, b.clone()));
                }
            }
        }

        None
    }

    fn combine_sub_chains_if_possible<W: Write>(&mut self, logger: &mut Logger<W>)
    {
        loop
        {
            let pair_to_combine = Self::find_pair(&mut self.sub_chains, &mut |a, b| {
                Self::check_sub_chains_to_combine(a, &b, logger)
            });

            match pair_to_combine
            {
                Some((a, b)) =>
                {
                    a.combine_with(&b);
                    self.delete_sub_chain(&b);
                },

                None => break,
            }
       }
    }

    fn on_unplaced_block<W: Write>(&mut self, block: Block, logger: &mut Logger<W>)
    {
        let sub_chain = self.add_block_to_sub_chain(block, logger);
        if self.main_chain.check_sub_chain(&sub_chain, logger) {
            self.delete_sub_chain(&sub_chain);
        }
        self.combine_sub_chains_if_possible(logger);
    }

    fn clean_sub_chains<W: Write>(&mut self, logger: &mut Logger<W>)
    {
        for sub_chain in self.sub_chains.clone() 
        {
            if self.main_chain.check_sub_chain(&sub_chain, logger) {
                self.delete_sub_chain(&sub_chain);
            }
        }
        self.combine_sub_chains_if_possible(logger);
    }

    pub fn buffer_block(&mut self, block: Block) -> Option<Block>
    {
        match self.buffer.push(block.clone())
        {
            Ok(buffered_block) => buffered_block,
            Err(_) => Some( block ),
        }
    }

    pub fn add<W: Write>(&mut self, block: Block, logger: &mut Logger<W>)
    {
        let buffered_block_or_none = self.buffer_block(block);
        if buffered_block_or_none.is_none() {
            return;
        }

        let buffered_block = buffered_block_or_none.unwrap();
        let existing_block = self.main_chain.block(buffered_block.block_id);
        if existing_block.is_some() && existing_block.unwrap() == buffered_block 
        {
            logger.log(LoggerLevel::Info, &format!("Ignoring duplicate block {}", buffered_block.block_id));
            self.clean_sub_chains(logger);
            return;
        }

        if self.is_valid_next_block(&buffered_block).is_err() 
        {
            self.on_unplaced_block(buffered_block, logger);
            return;
        }

        self.main_chain.add(buffered_block).unwrap();
        return;
    }

    pub fn block(&self, block_id: u64) -> Option<Block>
    {
        if self.buffer.base_id().is_none() || 
            block_id < self.buffer.base_id().unwrap()
        {
            self.main_chain.block(block_id)
        } 
        else 
        {
            self.buffer.block(block_id)
        }
    }

    pub fn top(&self) -> Option<Block>
    {
        match self.buffer.top()
        {
            Some(top) => Some( top ),
            None => self.main_chain.top(),
        }
    }

    pub fn top_id(&self) -> u64
    {
        match self.top()
        {
            Some(top) => top.block_id,
            None => 0,
        }
    }

    pub fn next_block_needed(&self) -> u64
    {
        let main_top = self.top_id();

        let mut max_sub_chain_len = 0;
        let mut max_sub_chain = None;
        for sub_chain in &self.sub_chains
        {
            if sub_chain.top.is_some() && sub_chain.top.unwrap() > max_sub_chain_len 
            {
                max_sub_chain_len = sub_chain.top.unwrap();
                max_sub_chain = Some( sub_chain );
            }
        }

        if max_sub_chain.is_none() || main_top >= max_sub_chain_len {
            main_top + 1
        } else {
            max_sub_chain.unwrap().bottom.unwrap() - 1
        }
    }

    pub fn lockup_wallet_status<W: Wallet>(&self, wallet: &W) -> WalletStatus
    {
        let mut status = WalletStatus
        {
            balance: 0.0,
            max_id: 0,
        };

        self.main_chain.lookup_chunks(&mut |chunk|
        {
            let change = chunk.wallet_status_change(wallet);
            status.balance += change.balance;
            status.max_id = std::cmp::max(status.max_id, change.max_id);
        });

        status
    }

}
