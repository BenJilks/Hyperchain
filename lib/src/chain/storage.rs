use crate::block::Block;

use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::error::Error;
use std::fs::File;

const CHUNK_SIZE: usize = 100;

#[derive(Serialize, Deserialize)]
struct Chunk
{
    blocks: Vec<Option<Block>>,
}

impl Default for Chunk
{
    fn default() -> Self
    {
        Self
        {
            blocks: vec![None; CHUNK_SIZE],
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Metadata
{
    next_top: u64,
}

impl Default for Metadata
{
    fn default() -> Self
    {
        Self
        {
            next_top: 0,
        }
    }
}

pub struct Storage
{
    path: PathBuf,
    metadata: Metadata,
}

fn chunk_id_for_block(block: &Block) -> usize
{
    block.block_id as usize / CHUNK_SIZE
}

fn load_chunk_file(path: PathBuf) -> Chunk
{
    match File::open(path)
    {
        Ok(file) => 
            match bincode::deserialize_from(file)
            {
                Ok(chunk) => chunk,
                Err(_) => Default::default(),
            },
        Err(_) => Default::default(),
    }
}

fn load_metadata(path: &PathBuf) -> Result<Metadata, Box<dyn Error>>
{
    match File::open(path.join("metadata"))
    {
        Ok(file) => Ok(serde_json::from_reader(file)?),
        Err(_) => Ok(Default::default()),
    }
}

impl Storage
{

    pub fn new(path: &PathBuf) -> Result<Self, Box<dyn Error>>
    {
        std::fs::create_dir_all(path)?;
        Ok(Self
        {
            path: path.clone(),
            metadata: load_metadata(path)?,
        })
    }

    #[cfg(test)]
    pub fn path(&self) -> &PathBuf
    {
        &self.path
    }

    fn save_metadata(&self)
    {
        match File::create(self.path.join("metadata"))
        {
            Ok(file) => { let _ = serde_json::to_writer(file, &self.metadata); },
            Err(_) => {},
        }
    }

    fn get_chunk_file_path(&self, id: usize) -> PathBuf
    {
        let file_name = format!("blk{}", id);
        self.path.join(file_name)
    }

    fn get_chunk(&self, id: usize) -> Chunk
    {
        let path = self.get_chunk_file_path(id);
        if !path.exists() {
           Default::default() 
        } else {
            load_chunk_file(path)
        }
    }

    fn store_chunk(&self, id: usize, chunk: Chunk)
    {
        let path = self.get_chunk_file_path(id);
        match File::create(path)
        {
            Ok(file) => { let _ = bincode::serialize_into(file, &chunk); },
            Err(_) => {},
        }
    }

    pub fn store(&mut self, block: Block)
    {
        self.metadata.next_top = std::cmp::max(self.metadata.next_top, block.block_id + 1);
        self.save_metadata();

        let chunk_id = chunk_id_for_block(&block);
        let mut chunk = self.get_chunk(chunk_id);
        let index = block.block_id as usize % CHUNK_SIZE;
        chunk.blocks[index] = Some(block);
        self.store_chunk(chunk_id, chunk);
    }

    pub fn get(&self, block_id: u64) -> Option<Block>
    {
        let chunk_id = block_id as usize / CHUNK_SIZE;
        let chunk = self.get_chunk(chunk_id);
        let index = block_id as usize % CHUNK_SIZE;
        chunk.blocks[index].clone()
    }

    pub fn next_top(&self) -> u64
    {
        self.metadata.next_top
    }

}

