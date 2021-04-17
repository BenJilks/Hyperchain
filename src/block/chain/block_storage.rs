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

#[cfg(test)]
mod tests
{
    use super::*;
    use super::super::chunk::SubChunk;

    fn store_blocks()
    {
        let x = BlockStorage::<SubChunk>::new(PathBuf::from("block_storage_tests_temp"));
        
        assert_eq!(x.set_block(Block
        {
            prev_hash: [0u8; 32],
            block_id: CHUNK_SIZE * 7 + 1,
            raward_to: [0u8; 32],
            pages: Vec::new(),
            transactions: Vec::new(),
            timestamp: 0,
            target: [0u8; 32],
            pow: 0,
        }).is_ok(), true);

        assert_eq!(x.set_block(Block
        {
            prev_hash: [0u8; 32],
            block_id: 1,
            raward_to: [0u8; 32],
            pages: Vec::new(),
            transactions: Vec::new(),
            timestamp: 0,
            target: [0u8; 32],
            pow: 0,
        }).is_ok(), true);
    }

    fn test_blocks()
    {
        let x = BlockStorage::<SubChunk>::new(PathBuf::from("block_storage_tests_temp"));
        assert_eq!(x.block(CHUNK_SIZE * 7 + 1).is_some(), true);
        assert_eq!(x.block(1).is_some(), true);
        assert_eq!(x.chunk(7).block(CHUNK_SIZE * 7 + 1).is_some(), true);
    }

    fn clean_up()
    {
        assert_eq!(std::fs::remove_dir_all(PathBuf::from("block_storage_tests_temp")).is_ok(), true);
    }

    #[test]
    fn test_block_storage()
    {
        store_blocks();
        test_blocks();
        clean_up();
    }

}
