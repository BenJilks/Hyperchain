mod chunk;
mod block_storage;
mod sub_chain;
use super::Block;
use crate::error::Error;
use crate::wallet::{Wallet, WalletStatus};
use crate::logger::{Logger, LoggerLevel};
use block_storage::BlockStorage;
use chunk::{CHUNK_SIZE, MainChunk};
use sub_chain::SubChain;

use std::fs;
use std::path::PathBuf;
use std::io::Write;
use rand::RngCore;

pub struct BlockChain
{
    path: PathBuf,
    main_chain: BlockStorage<MainChunk>,
    sub_chains: Vec<SubChain>,
}

impl BlockChain
{

    fn load_sub_chains(path: &PathBuf) -> Vec<SubChain>
    {
        std::fs::create_dir_all(&path).unwrap();

        let mut sub_chains = Vec::<SubChain>::new();
        for entry_or_error in std::fs::read_dir(path).unwrap()
        {
            if entry_or_error.is_err() {
                continue;
            }

            let entry = entry_or_error.unwrap();
            sub_chains.push(SubChain::from(entry.path()));
        }

        sub_chains
    }

    fn generate_sub_chain_id(&self) -> String
    {
        loop
        {
            let mut bytes = [0u8; 5];
            rand::thread_rng().fill_bytes(&mut bytes);
            
            let id = base_62::encode(&bytes);
            let sub_chain_path = self.path.join("sub_chains").join(&id);
            if !sub_chain_path.exists() {
                return id;
            }
        }
    }

    pub fn new<W: Write>(path: PathBuf, logger: &mut Logger<W>) -> Self
    {
        fs::create_dir_all(&path).unwrap();
        let mut chain = Self
        {
            path: path.clone(),
            main_chain: BlockStorage::new(path.join("main").clone()),
            sub_chains: Self::load_sub_chains(&path.join("sub_chains")),
        };

        logger.log(LoggerLevel::Info, &format!("Open blockchain '{:?}', with top {}", path, chain.top_id()));
        chain
    }

    fn is_valid_next_block(&self, block: &Block) -> Result<(), Error>
    {
        if block.block_id >= 1
        {
            if block.block_id != self.top_id() + 1 {
                return Err(Error::NotNextBlock);
            }

            if !block.validate_pow() {
                return Err(Error::InvalidPOW);
            }
        }

        block.validate(self)
    }

    fn add_block_to_sub_chain<W: Write>(&mut self, block: Block, logger: &mut Logger<W>) -> SubChain
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

        let new_id = self.generate_sub_chain_id();
        let mut new_sub_chain = SubChain::from(self.path.join("sub_chains").join(new_id));
        assert_eq!(new_sub_chain.add_block(&block), true);

        self.sub_chains.push(new_sub_chain.clone());
        return new_sub_chain;
    }

    fn delete_sub_chain(&mut self, sub_chain: &SubChain)
    {
        std::fs::remove_dir_all(&sub_chain.path).unwrap();
        let index = self.sub_chains.iter().position(|x| x == sub_chain).unwrap();
        self.sub_chains.remove(index);
    }

    fn replace_main_chain_top_with_sub_chain<W: Write>(&mut self, sub_chain: &SubChain, logger: &mut Logger<W>)
    {
        logger.log(LoggerLevel::Info, "Replace main chain top with sub chain");
        let new_main = BlockStorage::<MainChunk>::new(self.path.join(".temp"));

        let start_chunk = sub_chain.bottom.unwrap() / CHUNK_SIZE;
        for i in 0..start_chunk
        {
            let old_chunk_path = self.path.join("main").join(i.to_string());
            let new_chunk_path = self.path.join(".temp").join(i.to_string());
            std::fs::copy(old_chunk_path, new_chunk_path).unwrap();

            // let chunk = new_main.chunk(i);
            // TODO: Apply pages
        }

        let start_block_id = std::cmp::max(start_chunk * CHUNK_SIZE, 1);
        for i in start_block_id..=sub_chain.top.unwrap() 
        {
            let block = 
                if sub_chain.bottom.unwrap() > i {
                    self.block(i).unwrap()
                } else {
                    sub_chain.block(i).unwrap()
                };

            new_main.set_block(block).unwrap();
        }

        logger.log(LoggerLevel::Info, "Replace main with new chain and delete old sub chain");
        std::fs::remove_dir_all(self.path.join("main")).unwrap();
        std::fs::rename(self.path.join(".temp"), self.path.join("main")).unwrap();
        self.delete_sub_chain(sub_chain);
    }

    fn check_sub_chain<W: Write>(&mut self, sub_chain: &SubChain, logger: &mut Logger<W>)
    {
        if sub_chain.top.is_none() || sub_chain.top.unwrap() < self.top_id() {
            return;
        }

        if sub_chain.bottom.is_none() || sub_chain.bottom.unwrap() > self.top_id() + 1 {
            return;
        }

        let bottom_or_none = sub_chain.block(sub_chain.bottom.unwrap());
        if bottom_or_none.is_none() {
            return;
        }

        let bottom = bottom_or_none.unwrap();
        if bottom.block_id > 1
        {
            let prev_in_main_chain = self.block(bottom.block_id - 1).unwrap();
            if bottom.is_next_block(&prev_in_main_chain).is_err() {
                return;
            }
        }

        // This sub chain is now the top of a new main chain
        self.replace_main_chain_top_with_sub_chain(sub_chain, logger);
    }

    fn check_sub_chains_to_combine<W: Write>(a: &mut SubChain, b: &SubChain, logger: &mut Logger<W>) -> bool
    {
        if SubChain::can_combine(a, b) 
        {
            logger.log(LoggerLevel::Info, &format!("Combine {} -> {} with {} -> {}", 
                a.bottom.unwrap(), a.top.unwrap(),
                b.bottom.unwrap(), b.top.unwrap()));

            a.combine_with(&b);
            return true;
        }

        return false;
    }

    fn combine_sub_chains_if_possible<W: Write>(&mut self, logger: &mut Logger<W>)
    {
        let mut did_combine;
        loop
        {
            did_combine = false;

            for b in self.sub_chains.clone()
            {
                for a in &mut self.sub_chains
                {
                    if a == &b {
                        continue;
                    }

                    if Self::check_sub_chains_to_combine(a, &b, logger) 
                    {
                        self.delete_sub_chain(&b);
                        did_combine = true;
                        break;
                    }
                }

                if did_combine {
                    break;
                }
            }

            if !did_combine {
                break;
            }
        }
    }

    fn on_unplaced_block<W: Write>(&mut self, block: Block, logger: &mut Logger<W>)
    {
        let sub_chain = self.add_block_to_sub_chain(block, logger);
        self.check_sub_chain(&sub_chain, logger);
        self.combine_sub_chains_if_possible(logger);
    }

    fn clean_sub_chains<W: Write>(&mut self, logger: &mut Logger<W>)
    {
        for sub_chain in self.sub_chains.clone() {
            self.check_sub_chain(&sub_chain, logger);
        }
        self.combine_sub_chains_if_possible(logger);
    }

    pub fn add<W: Write>(&mut self, block: Block, logger: &mut Logger<W>) -> bool
    {
        let existing_block = self.block(block.block_id);
        if existing_block.is_some() && existing_block.unwrap() == block 
        {
            logger.log(LoggerLevel::Info, &format!("Ignoring duplicate block {}", block.block_id));
            self.clean_sub_chains(logger);
            return false;
        }

        if self.is_valid_next_block(&block).is_err() 
        {
            self.on_unplaced_block(block, logger);
            return false;
        }

        self.main_chain.set_block(block).unwrap();
        self.clean_sub_chains(logger);
        return true;
    }

    pub fn block(&self, block_id: u64) -> Option<Block>
    {
        self.main_chain.block(block_id)
    }

    fn lookup_chunks<F>(&self, callback: &mut F)
        where F: FnMut(&MainChunk)
    {
        let mut chunk_id = 0u64;
        loop
        {
            let chunk = self.main_chain.chunk(chunk_id);
            if chunk.top().is_none() {
                break;
            }

            callback(&chunk);
            chunk_id += 1;
        }
    }

    pub fn top(&self) -> Option<Block>
    {
        let mut top_chunk = None;
        self.lookup_chunks(&mut |chunk: &MainChunk|
        {
            top_chunk = Some( chunk.clone() );
        });

        
        if top_chunk.is_none() {
            None
        } else {
            top_chunk.unwrap().top()
        }
    }

    pub fn top_id(&self) -> u64
    {
        match self.top()
        {
            Some(block) => block.block_id,
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

        self.lookup_chunks(&mut |chunk|
        {
            let change = chunk.wallet_status_change(wallet);
            status.balance += change.balance;
            status.max_id = std::cmp::max(status.max_id, change.max_id);
        });

        status
    }

}
