/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use serde::{Serialize, Deserialize};
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::error::Error;
use std::fs::File;

const CHUNK_SIZE: usize = 100;

#[derive(Clone, Serialize, Deserialize)]
struct Chunk<T>
{
    data: Vec<Option<T>>,
}

impl<T> Default for Chunk<T>
    where T: Clone
{
    fn default() -> Self
    {
        Self
        {
            data: vec![None; CHUNK_SIZE],
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

pub struct Storage<T>
{
    path: PathBuf,
    metadata: Metadata,
    cache: Option<(usize, Chunk<T>)>,
}

fn load_chunk_file<T>(path: PathBuf) -> Chunk<T>
    where T: Clone + DeserializeOwned
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
    match File::open(path.join("metadata.json"))
    {
        Ok(file) => Ok(serde_json::from_reader(file)?),
        Err(_) => Ok(Default::default()),
    }
}

impl<T> Storage<T>
    where T: Clone + Serialize + DeserializeOwned
{

    pub fn new(path: &PathBuf) -> Result<Self, Box<dyn Error>>
    {
        std::fs::create_dir_all(path)?;
        Ok(Self
        {
            path: path.clone(),
            metadata: load_metadata(path)?,
            cache: None,
        })
    }

    #[cfg(test)]
    pub fn path(&self) -> &PathBuf
    {
        &self.path
    }

    fn save_metadata(&self)
    {
        match File::create(self.path.join("metadata.json"))
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

    fn get_chunk(&mut self, id: usize) -> Chunk<T>
    {
        if self.cache.is_some()
        {
            let (cache_id, cache_chunk) = self.cache.as_ref().unwrap();
            if *cache_id == id {
                return cache_chunk.clone();
            }
        }

        let path = self.get_chunk_file_path(id);
        let chunk = 
            if !path.exists() {
                Default::default() 
            } else {
                load_chunk_file(path)
            };

        self.cache = Some((id, chunk.clone()));
        return chunk;
    }

    fn store_chunk(&mut self, id: usize, chunk: Chunk<T>)
    {
        let path = self.get_chunk_file_path(id);
        match File::create(path)
        {
            Ok(file) => { let _ = bincode::serialize_into(file, &chunk); },
            Err(_) => {},
        }

        self.cache = Some((id, chunk.clone()));
    }

    pub fn store(&mut self, block_id: u64, block: T)
    {
        self.metadata.next_top = std::cmp::max(self.metadata.next_top, block_id + 1);
        self.save_metadata();

        let chunk_id = block_id as usize / CHUNK_SIZE;
        let index = block_id as usize % CHUNK_SIZE;
        let mut chunk = self.get_chunk(chunk_id);
        chunk.data[index] = Some(block);
        self.store_chunk(chunk_id, chunk);
    }

    pub fn truncate(&mut self, new_size: u64)
    {
        self.metadata.next_top = new_size;
        self.save_metadata();
    }

    pub fn get(&mut self, block_id: u64) -> Option<T>
    {
        let chunk_id = block_id as usize / CHUNK_SIZE;
        let chunk = self.get_chunk(chunk_id);
        let index = block_id as usize % CHUNK_SIZE;
        chunk.data[index].clone()
    }

    pub fn next_top(&self) -> u64
    {
        self.metadata.next_top
    }

}

