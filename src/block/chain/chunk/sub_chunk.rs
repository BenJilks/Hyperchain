use super::{Chunk, CHUNK_SIZE};
use crate::block::Block;
use crate::error::Error;

use std::path::PathBuf;
use std::fs::File;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct SubChunk
{
    chunk_id: u64,
    blocks: Vec<Option<Block>>,
}

impl SubChunk
{

    fn new(path: PathBuf, chunk_id: u64) -> Self
    {
        std::fs::create_dir_all(&path).unwrap();

        Self
        {
            chunk_id: chunk_id,
            blocks: vec![None; CHUNK_SIZE as usize],
        }
    }

}

impl Chunk for SubChunk
{

    fn from(path: PathBuf, chunk_id: u64) -> Self
    {
        let chunk_path = path.join(chunk_id.to_string());
        if !chunk_path.exists() {
            return Self::new(path, chunk_id);
        }

        let chunk_file = File::open(chunk_path).unwrap();
        bincode::deserialize_from::<File, Self>(chunk_file).unwrap()
    }

    fn write(&self, path: PathBuf)
    {
        let chunk_path = path.join(self.chunk_id.to_string());
        let chunk_file = File::create(chunk_path).unwrap();
        bincode::serialize_into(chunk_file, self).unwrap();
    }

    fn block(&self, block_id: u64) -> Option<Block>
    {
        let local_id = (block_id % CHUNK_SIZE) as usize;
        self.blocks[local_id].clone()
    }

    fn set_block(&mut self, block: Block) -> Result<(), Error>
    {
        let local_id = (block.block_id % CHUNK_SIZE) as usize;
        self.blocks[local_id] = Some( block );
        Ok(())
    }

}
