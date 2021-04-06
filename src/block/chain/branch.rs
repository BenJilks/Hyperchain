use crate::block::{Block, Page, DataFormat};
use crate::error::Error;
use crate::wallet::{PublicWallet, Wallet};
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

    fn find_top_index(blocks_path: &PathBuf) -> u64
    {
        let mut id = 1;
        loop
        {
            // FIXME: This is not good, not good at all :(
            if !blocks_path.join(id.to_string()).exists()
            {
                if id <= 1 {
                    return 0;
                }

                return id - 1;
            }

            id += 1;
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
        let file = File::open(self.blocks_path.join(id.to_string()));
        if file.is_err() {
            return None;
        }

        let mut bytes = Vec::<u8>::new();
        file.unwrap().read_to_end(&mut bytes).unwrap();
        return Block::from_bytes(&bytes);
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
        if self.top_index == 0 {
            None
        } else {
            self.block(self.top_index)
        }
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

        let bytes = block.as_bytes()?;
        let block_path = self.blocks_path.join(block.block_id.to_string());
        let mut file = io_error(File::create(block_path))?;
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

    pub fn lookup<Callback>(&self, callback: &mut Callback)
        where Callback: FnMut(&Block)
    {
        let mut id = 1;
        loop
        {
            // FIXME: This is not good, not good at all :(
            if !self.blocks_path.join(id.to_string()).exists() {
                break;
            }

            let block = self.block(id).unwrap();
            callback(&block);
            id += 1;
        }
    }

}
