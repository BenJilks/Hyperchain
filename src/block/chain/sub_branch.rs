use super::BlockStorage;
use super::chunk::SubChunk;
use crate::block::Block;

use std::path::PathBuf;
use std::fs::File;
use serde::{Serialize, Deserialize};
use rand::RngCore;

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct SubBranch
{
    pub path: PathBuf,
    pub bottom: Option<u64>,
    pub top: Option<u64>,
}

impl SubBranch
{

    pub fn load_sub_branches(path: &PathBuf) -> Vec<SubBranch>
    {
        std::fs::create_dir_all(&path).unwrap();

        let mut sub_branches = Vec::<SubBranch>::new();
        for entry_or_error in std::fs::read_dir(path).unwrap()
        {
            if entry_or_error.is_err() {
                continue;
            }

            let entry = entry_or_error.unwrap();
            sub_branches.push(SubBranch::from(entry.path()));
        }

        sub_branches
    }

    pub fn generate_sub_branch_id(path: &PathBuf) -> String
    {
        loop
        {
            let mut bytes = [0u8; 5];
            rand::thread_rng().fill_bytes(&mut bytes);
            
            let id = base_62::encode(&bytes);
            let sub_branch_path = path.join("sub_branches").join(&id);
            if !sub_branch_path.exists() {
                return id;
            }
        }
    }

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

    pub fn can_combine(a: &Self, b: &Self) -> bool
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

    pub fn combine_with(&mut self, other: &Self)
    {
        assert_eq!(Self::can_combine(self, other), true);

        if self.bottom.unwrap() < other.top.unwrap() 
        {
            for i in other.bottom.unwrap()..=other.top.unwrap() {
                self.add_block(&other.block(i).unwrap());
            }
        }
        else
        {
            for i in (other.bottom.unwrap()..=other.top.unwrap()).rev() {
                self.add_block(&other.block(i).unwrap());
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
