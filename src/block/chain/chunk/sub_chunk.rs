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

#[cfg(test)]
mod tests
{
    use super::*;

    fn write_blocks_to_chunk()
    {
        let mut a: SubChunk = Chunk::from(PathBuf::from("sub_chunk_tests_temp"), 0);
        assert_eq!(a.set_block(Block
        {
            prev_hash: [0u8; 32],
            block_id: 0,
            raward_to: [0u8; 32],
            pages: Vec::new(),
            transactions: Vec::new(),
            timestamp: 0,
            target: [0u8; 32],
            pow: 0,
        }).is_ok(), true);

        assert_eq!(a.set_block(Block
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

        a.write(PathBuf::from("sub_chunk_tests_temp"));
    }

    fn test_chunk_has_blocks()
    {
        let a: SubChunk = Chunk::from(PathBuf::from("sub_chunk_tests_temp"), 0);
        assert_eq!(a.blocks.iter().filter(|x| x.is_some()).count(), 2);

        let test_block = |block_or_none: &Option<Block>, id: u64|
        {
            assert_eq!(block_or_none.is_some(), true);

            let block = block_or_none.clone().unwrap();
            assert_eq!(block.prev_hash, [0u8; 32]);
            assert_eq!(block.block_id, id);
            assert_eq!(block.raward_to, [0u8; 32]);
            assert_eq!(block.pages, Vec::new());
            assert_eq!(block.transactions, Vec::new());
            assert_eq!(block.timestamp, 0);
            assert_eq!(block.target, [0u8; 32]);
            assert_eq!(block.pow, 0);
        };

        test_block(&a.blocks[0], 0);
        test_block(&a.blocks[1], 1);
    }

    fn clean_up()
    {
        assert_eq!(std::fs::remove_dir_all(PathBuf::from("sub_chunk_tests_temp")).is_ok(), true);
    }

    #[test]
    fn test_sub_chunk()
    {
        write_blocks_to_chunk();
        test_chunk_has_blocks();
        clean_up();
    }

}
