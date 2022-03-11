pub mod page;
pub mod data_unit;
use data_unit::DataUnit;
use crate::transaction::Transaction;
use crate::transaction::page::Page;
use crate::hash::Hash;
use std::path::{Path, PathBuf};
use std::fs::File;
use std::collections::{HashSet, HashMap};
use std::error::Error;

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

    pub fn for_page_updates(&self, updates: &Vec<Transaction<Page>>) 
        -> Result<HashMap<Hash, DataUnit>, Box<dyn Error>>
    {
        let mut data = HashMap::new();
        for update in updates 
        {
            let data_unit = self.get_data_unit(update)?;
            let hash = update.hash()?;
            data.insert(hash, data_unit);
        }

        Ok(data)
    }

    pub fn store(&self, id: &Hash, data: &[u8]) 
        -> Result<(), Box<dyn Error>>
    {
        let file_name = format!("{}", id);
        let file = File::create(self.path.join(file_name))?;
        bincode::serialize_into(file, &data)?;
        Ok(())
    }

    pub fn store_data_unit(&self, data_unit: &DataUnit)
        -> Result<(), Box<dyn Error>>
    {
        for (chunk, hash) in data_unit.chunks()? {
            self.store(&hash, &chunk)?;
        }
        Ok(())
    }

    pub fn get(&self, id: &Hash) -> Result<Vec<u8>, Box<dyn Error>>
    {
        let file_name = format!("{}", id);
        let file = File::open(self.path.join(file_name))?;
        Ok(bincode::deserialize_from(file)?)
    }

    pub fn get_data_unit(&self, transaction: &Transaction<Page>)
        -> Result<DataUnit, Box<dyn Error>>
    {
        let mut data_unit_bytes = Vec::new();
        for chunk_hash in &transaction.header.content.data_hashes
        {
            let mut chunk = self.get(chunk_hash)?;
            data_unit_bytes.append(&mut chunk);
        }

        Ok(bincode::deserialize(&data_unit_bytes)?)
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use page::CreatePageData;

    impl DataStore
    {
        pub fn open_temp() -> Self
        {
            let path = std::env::temp_dir().join(rand::random::<u32>().to_string());
            Self::open(&path).unwrap()
        }
    }

    impl Drop for DataStore
    {
        fn drop(&mut self)
        {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn test_data_store()
    {
        let data_store = DataStore::open_temp();
        let test_unit = DataUnit::CreatePage(
            CreatePageData::new("index.html".to_owned(), Vec::new()));

        let test_data = bincode::serialize(&test_unit).unwrap();
        data_store.store(&Hash::empty(), &test_data).unwrap();
        assert_eq!(data_store.get(&Hash::empty()).unwrap(), test_data);
    }

}

