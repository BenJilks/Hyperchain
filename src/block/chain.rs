use super::{Block, Page, DataFormat};
use crate::wallet::{Wallet, PublicWallet};
use std::fs::{self, File};
use std::io::{Write, Read};
use std::path::PathBuf;

pub struct BlockChain
{
    path: PathBuf,
    blocks_path: PathBuf,
    sites_path: PathBuf,
}

impl BlockChain
{

    pub fn new(path: PathBuf) -> Self
    {
        let blocks_path = path.join("blocks");
        let sites_path = path.join("sites");
        fs::create_dir_all(&path).unwrap();
        fs::create_dir_all(&blocks_path).unwrap();
        fs::create_dir_all(&sites_path).unwrap();

        Self
        {
            path,
            blocks_path,
            sites_path,
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

    pub fn top(&self) -> Option<Block>
    {
        let mut id = 1;
        loop
        {
            // FIXME: This is not good, not good at all :(
            if !self.blocks_path.join(id.to_string()).exists()
            {
                if id <= 1 {
                    return None;
                }

                return self.block(id - 1);
            }

            id += 1;
        }
    }

    fn apply_page(&self, page: &Page) -> std::io::Result<()>
    {
        let owner = PublicWallet::from_public_key(page.header.site_id);
        let path = self.sites_path
            .join(base_62::encode(&owner.get_address()))
            .join(&page.header.page_name);
        let directroy = path.parent().unwrap();

        if !directroy.exists() {
            std::fs::create_dir_all(&directroy)?;
        }

        match DataFormat::from_u8(page.header.data_format)
        {
            Some( DataFormat::NewRaw ) =>
            {
                let mut file = File::create(&path)?;
                file.write(&page.header.page_data)?;
            },

            Some( DataFormat::DiffRaw ) =>
            {
                let mut buffer = Vec::<u8>::new();
                {
                    let file = File::open(&path)?;
                    let mut reader = bipatch::Reader::new(&page.header.page_data[..], file).unwrap();
                    reader.read_to_end(&mut buffer)?;
                }

                let mut file = File::create(&path)?;
                file.write(&buffer)?;
            },

            None => panic!("Invlid data format!"),
        }

        Ok(())
    }

    pub fn add(&self, block: &Block) -> std::io::Result<()>
    {
        if block.block_id > 1
        {
            if !block.validate(self) {
                return Ok(()); // FIXME: This should be an error
            }
        }

        if !block.validate_pow() {
            return Ok(()); // FIXME: This should be an error
        }

        let bytes = block.as_bytes();
        if bytes.is_none() {
            return Ok(()); // FIXME: This should be an error
        }

        let mut file = File::create(self.blocks_path.join(block.block_id.to_string()))?;
        file.write(&bytes.unwrap())?;

        for page in &block.pages {
            self.apply_page(page)?;
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

}
