use super::chunk::{Chunk, CHUNK_SIZE};
use crate::block::Block;
use crate::error::Error;

use std::path::PathBuf;

pub struct BlockStorage<C>
    where C: Chunk
{
    path: PathBuf,
    _chunk: Option<C>,
}

impl<C> BlockStorage<C>
    where C: Chunk
{

    pub fn new(path: PathBuf) -> Self
    {
        std::fs::create_dir_all(&path).unwrap();

        Self
        {
            path: path,
            _chunk: None,
        }
    }

    fn chunk_id_from_block_id(block_id: u64) -> u64
    {
        block_id / CHUNK_SIZE
    }

    pub fn chunk(&self, chunk_id: u64) -> C
    {
        C::from(self.path.clone(), chunk_id)
    }

    pub fn block(&self, block_id: u64) -> Option<Block>
    {
        let chunk_id = Self::chunk_id_from_block_id(block_id);
        self.chunk(chunk_id).block(block_id)
    }

    pub fn set_block(&self, block: Block) -> Result<(), Error>
    {
        let chunk_id = Self::chunk_id_from_block_id(block.block_id);
        let mut chunk = self.chunk(chunk_id);
        let result = chunk.set_block(block);
        chunk.write(self.path.clone());

        result
    }

}
