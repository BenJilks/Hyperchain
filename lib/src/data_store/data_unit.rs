use super::page::CreatePageData;
use crate::config::PAGE_CHUNK_SIZE;
use crate::hash::Hash;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DataUnit
{
    CreatePage(CreatePageData),
}

impl DataUnit
{

    pub fn chunks(&self) -> Result<Vec<(Vec<u8>, Hash)>, Box<dyn Error>>
    {
        let data = bincode::serialize(self)?;
        
        let mut hashes = Vec::new();
        let mut chunk_start: usize = 0;
        while chunk_start < data.len()
        {
            let chunk_end = std::cmp::min(chunk_start + PAGE_CHUNK_SIZE, data.len());
            let chunk = &data[chunk_start..chunk_end];
            
            let mut hasher = Sha256::new();
            hasher.update(chunk);

            let hash = Hash::from(&hasher.finalize());
            hashes.push((chunk.to_vec(), hash));
            
            chunk_start = chunk_end;
        }

        Ok(hashes)
    }

    pub fn hashes(&self) -> Result<Vec<Hash>, Box<dyn Error>>
    {
        Ok(self.chunks()?
            .iter()
            .map(|(_, hash)| *hash)
            .collect())
    }

    pub fn len(&self) -> Result<u32, Box<dyn Error>>
    {
        let data = bincode::serialize(self)?;
        Ok(data.len() as u32)
    }

}

