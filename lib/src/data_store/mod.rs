pub mod page;
use page::CreatePageData;
use crate::config::{Hash, HASH_LEN, PAGE_CHUNK_SIZE};

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::path::PathBuf;
use std::fs::File;
use std::error::Error;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum DataUnit
{
    CreatePage(CreatePageData),
}

impl DataUnit
{

    pub fn get_hashes(&self) -> Result<Vec<Hash>, Box<dyn Error>>
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

            let hash_vec = hasher.finalize().to_vec();
            let hash = slice_as_array!(&hash_vec, [u8; HASH_LEN]);
            hashes.push(*hash.unwrap());
            
            chunk_start = chunk_end;
        }

        Ok(hashes)
    }

    pub fn len(&self) -> Result<u32, Box<dyn Error>>
    {
        let data = bincode::serialize(self)?;
        Ok(data.len() as u32)
    }

}

pub struct DataStore
{
    path: PathBuf,
}

impl DataStore
{

    pub fn open(path: &PathBuf) -> Result<Self, Box<dyn Error>>
    {
        std::fs::create_dir_all(path)?;
        Ok(Self
        {
            path: path.clone(),
        })
    }

    pub fn store(&self, transaction_id: &[u8], data: &DataUnit) 
        -> Result<(), Box<dyn Error>>
    {
        let file_name = base_62::encode(transaction_id);
        let file = File::create(self.path.join(file_name))?;
        bincode::serialize_into(file, &data)?;
        Ok(())
    }

    pub fn get(&self, transaction_id: &[u8]) -> Result<DataUnit, Box<dyn Error>>
    {
        let file_name = base_62::encode(transaction_id);
        let file = File::open(self.path.join(file_name))?;
        Ok(bincode::deserialize_from(file)?)
    }

}
