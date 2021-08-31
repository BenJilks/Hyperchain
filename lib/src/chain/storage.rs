use crate::block::Block;

use std::collections::HashMap;
use std::path::PathBuf;

pub struct Storage
{
    _path: PathBuf,
    blocks: HashMap<u64, Block>,
    next_top: u64,
}

impl Storage
{

    pub fn new(path: &PathBuf) -> Self
    {
        Self
        {
            _path: path.clone(),
            blocks: HashMap::new(),
            next_top: 0,
        }
    }

    pub fn store(&mut self, block: Block)
    {
        self.next_top = std::cmp::max(self.next_top, block.block_id + 1);
        self.blocks.insert(block.block_id, block);
    }

    pub fn get(&self, block_id: u64) -> Option<Block>
    {
        match self.blocks.get(&block_id)
        {
            Some(block) => Some(block.clone()),
            None => None,
        }
    }

    pub fn next_top(&self) -> u64
    {
        self.next_top
    }

}
