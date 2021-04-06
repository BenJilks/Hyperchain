use super::chunk::{BlockChainChunk, CHUNK_SIZE};
use crate::block::{Block, Page, DataFormat};
use crate::error::Error;
use crate::wallet::{PublicWallet, Wallet, WalletStatus};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

fn io_error<T>(result: std::io::Result<T>) -> Result<T, Error>
{
    if result.is_ok() {
        Ok(result.ok().unwrap())
    } else {
        Err(Error::Other(result.err().unwrap().to_string()))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BlockChainBranch
{
    pub path: PathBuf,
    blocks_path: PathBuf,
    sites_path: PathBuf,

    pub top_index: u64,
}

impl BlockChainBranch
{

    fn chunk(blocks_path: &PathBuf, chunk_id: u64) -> Option<BlockChainChunk>
    {
        let path = blocks_path.join(chunk_id.to_string());
        if !path.exists() {
            return None;
        }

        let file = File::open(&path).unwrap();
        Some( bincode::deserialize_from(file).unwrap() )
    }

    fn find_top_index(blocks_path: &PathBuf) -> u64
    {
        let mut chunk_id = 0;
        loop
        {
            // FIXME: This is not good, not good at all :(
            if !blocks_path.join(chunk_id.to_string()).exists()
            {
                if chunk_id == 0 {
                    return 0;
                }

                let chunk = Self::chunk(blocks_path, chunk_id - 1).unwrap();
                return chunk.top_index();
            }

            chunk_id += 1;
        }
    }

    pub fn new(path: PathBuf) -> Self
    {
        let blocks_path = path.join("blocks");
        let sites_path = path.join("sites");
        fs::create_dir_all(&blocks_path).unwrap();
        fs::create_dir_all(&sites_path).unwrap();

        let top_index = Self::find_top_index(&blocks_path);
        Self
        {
            path,
            blocks_path,
            sites_path,
            top_index,
        }
    }

    pub fn block(&self, id: u64) -> Option<Block>
    {
        if id == 0 {
            return None;
        }

        let chunk_id = id / CHUNK_SIZE;
        let chunk = Self::chunk(&self.blocks_path, chunk_id);
        chunk?.block(id)
    }

    pub fn page<W: Wallet>(&self, site: &W, page_name: &str) -> Option<File>
    {
        let path = self.sites_path
            .join(base_62::encode(&site.get_address()))
            .join(page_name);

        if path.exists() {
            Some( File::open(path).unwrap() )
        } else {
            None
        }
    }

    pub fn top(&self) -> Option<Block>
    {
        self.block(self.top_index)
    }

    pub fn add(&mut self, block: &Block) -> Result<(), Error>
    {
        if block.block_id != self.top_index + 1 {
            return Err(Error::NotNextBlock);
        }

        if block.block_id > 1 {
            block.validate(self)?
        }

        if !block.validate_pow() {
            return Err(Error::InvalidPOW);
        }

        let chunk_id = block.block_id / CHUNK_SIZE;
        let chunk_or_none = Self::chunk(&self.blocks_path, chunk_id);
        let mut chunk = 
            if chunk_or_none.is_some() {
                chunk_or_none.unwrap()
            } else {
                BlockChainChunk::new(chunk_id)
            };
        chunk.add(block);

        let bytes = bincode::serialize(&chunk).unwrap();
        let chunk_path = self.blocks_path.join(chunk_id.to_string());
        let mut file = io_error(File::create(chunk_path))?;
        io_error(file.write(&bytes))?;

        for page in &block.pages {
            self.apply_page(page)?;
        }

        self.top_index += 1;
        Ok(())
    }

    fn apply_page(&self, page: &Page) -> Result<(), Error>
    {
        let owner = PublicWallet::from_public_key(page.header.site_id);
        let path = self.sites_path
            .join(base_62::encode(&owner.get_address()))
            .join(&page.header.page_name);
        let directroy = path.parent().unwrap();

        if !directroy.exists() {
            io_error(std::fs::create_dir_all(&directroy))?;
        }

        match DataFormat::from_u8(page.header.data_format)
        {
            Some( DataFormat::NewRaw ) =>
            {
                let mut file = io_error(File::create(&path))?;
                io_error(file.write(&page.header.page_data))?;
            },

            Some( DataFormat::DiffRaw ) =>
            {
                let mut buffer = Vec::<u8>::new();
                {
                    let file = io_error(File::open(&path))?;
                    let mut reader = bipatch::Reader::new(&page.header.page_data[..], file).unwrap();
                    io_error(reader.read_to_end(&mut buffer))?;
                }

                let mut file = io_error(File::create(&path))?;
                io_error(file.write(&buffer))?;
            },

            None => panic!("Invlid data format!"),
        }

        Ok(())
    }

    fn lookup_chunks<Callback>(&self, callback: &mut Callback)
        where Callback: FnMut(&BlockChainChunk)
    {
        let mut chunk_id = 0;
        loop
        {
            // FIXME: Still not good but better :|
            match Self::chunk(&self.blocks_path, chunk_id)
            {
                Some(chunk) => callback(&chunk),
                None => break,
            }

            chunk_id += 1;
        }
    }

    pub fn lockup_wallet_status<W: Wallet>(&self, wallet: &W) -> WalletStatus
    {
        let mut status = WalletStatus
        {
            balance: 0f64,
            max_id: 0,
        };

        self.lookup_chunks(&mut |chunk: &BlockChainChunk| 
        {
            let status_change = chunk.wallet_status_change(wallet);
            status.balance += status_change.balance;
            status.max_id = std::cmp::max(status.max_id, status_change.max_id);
        });

        status
    }

}
