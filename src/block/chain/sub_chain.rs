use super::BlockStorage;
use super::chunk::SubChunk;
use crate::block::Block;

use std::path::PathBuf;
use std::fs::File;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct SubChain
{
    pub path: PathBuf,
    pub bottom: Option<u64>,
    pub top: Option<u64>,
}

impl SubChain
{

    fn new(path: PathBuf) -> Self
    {
        Self
        {
            path: path.clone(),
            bottom: None,
            top: None,
        }
    }

    pub fn from(path: PathBuf) -> Self
    {
        if !path.exists() {
            return Self::new(path);
        }

        let file = File::open(&path.join("header")).unwrap();
        bincode::deserialize_from::<File, Self>(file).unwrap()
    }

    pub fn write(&self)
    {
        let file = File::create(&self.path.join("header")).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    pub fn block(&self, block_id: u64) -> Option<Block>
    {
        let storage = BlockStorage::<SubChunk>::new(self.path.clone());
        storage.block(block_id)
    }

    pub fn can_combine(a: &SubChain, b: &SubChain) -> bool
    {
        if a.top.is_none() || b.bottom.is_none() {
            return false;
        }
        if a.top.unwrap() + 1 == b.bottom.unwrap() {
            return true;
        }

        if b.top.is_none() || a.bottom.is_none() {
            return false;
        }
        if b.top.unwrap() + 1 == a.bottom.unwrap() {
            return true;
        }

        return false;
    }

    pub fn combine_with(&mut self, other: &SubChain)
    {
        assert_eq!(Self::can_combine(self, other), true);

        if self.bottom.unwrap() < other.top.unwrap() 
        {
            for i in other.bottom.unwrap()..=other.top.unwrap() {
                assert_eq!(self.add_block(&other.block(i).unwrap()), true);
            }
        }
        else
        {
            for i in (other.bottom.unwrap()..=other.top.unwrap()).rev() {
                assert_eq!(self.add_block(&other.block(i).unwrap()), true);
            }
        }
    }

    pub fn add_block(&mut self, block: &Block) -> bool
    {
        let storage = BlockStorage::<SubChunk>::new(self.path.clone());

        if self.bottom.is_none() || self.top.is_none() 
        {
            storage.set_block(block.clone()).unwrap();
            self.top = Some( block.block_id );
            self.bottom = Some( block.block_id );
            self.write();
            return true;
        }

        if block.block_id == self.top.unwrap() + 1 
        {
            let last = storage.block(self.top.unwrap()).unwrap();
            if block.is_next_block(&last).is_err() {
                return false;
            }

            storage.set_block(block.clone()).unwrap();
            self.top = Some( block.block_id );
            self.write();
            return true;
        }
        
        if block.block_id == self.bottom.unwrap() - 1
        {
            let last = storage.block(self.bottom.unwrap()).unwrap();
            if last.is_next_block(block).is_err() {
                return false;
            }

            storage.set_block(block.clone()).unwrap();
            self.bottom = Some( block.block_id );
            self.write();
            return true;
        }

        false
    }

}
