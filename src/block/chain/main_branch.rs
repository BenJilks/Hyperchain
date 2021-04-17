use super::block_storage::BlockStorage;
use super::chunk::{MainChunk, CHUNK_SIZE};
use super::sub_branch::SubBranch;
use crate::block::Block;
use crate::error::Error;
use crate::logger::{Logger, LoggerLevel};

use std::path::PathBuf;
use std::io::Write;

pub struct MainBranch
{
    path: PathBuf,
    storage: BlockStorage<MainChunk>,
}

impl MainBranch
{

    pub fn new(path: PathBuf) -> Self
    {
        Self
        {
            path: path.clone(),
            storage: BlockStorage::new(path.join("main").clone()),
        }
    }

    pub fn add(&self, block: Block) -> Result<(), Error>
    {
        self.storage.set_block(block)
    }

    pub fn block(&self, block_id: u64) -> Option<Block>
    {
        self.storage.block(block_id)
    }

    pub fn top(&self) -> Option<Block>
    {
        let mut top_chunk = None;
        self.lookup_chunks(&mut |chunk: &MainChunk| {
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

    fn replace_main_chain_top_with_sub_chain<W: Write>(&mut self, sub_chain: &SubBranch, logger: &mut Logger<W>)
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
    }

    pub fn check_sub_chain<W: Write>(&mut self, sub_chain: &SubBranch, logger: &mut Logger<W>) -> bool
    {
        if sub_chain.top.is_none() || sub_chain.top.unwrap() < self.top_id() {
            return false;
        }

        if sub_chain.bottom.is_none() || sub_chain.bottom.unwrap() > self.top_id() + 1 {
            return false;
        }

        let bottom_or_none = sub_chain.block(sub_chain.bottom.unwrap());
        if bottom_or_none.is_none() {
            return false;
        }

        let bottom = bottom_or_none.unwrap();
        if bottom.block_id > 1
        {
            let prev_in_main_chain = self.block(bottom.block_id - 1).unwrap();
            if bottom.is_next_block(&prev_in_main_chain).is_err() {
                return false;
            }
        }

        // This sub chain is now the top of a new main chain
        self.replace_main_chain_top_with_sub_chain(sub_chain, logger);
        return true;
    }

    pub fn lookup_chunks<F>(&self, callback: &mut F)
        where F: FnMut(&MainChunk)
    {
        let mut chunk_id = 0u64;
        loop
        {
            let chunk = self.storage.chunk(chunk_id);
            if chunk.top().is_none() {
                break;
            }

            callback(&chunk);
            chunk_id += 1;
        }
    }

}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn test_main_chain()
    {
        
    }

}
