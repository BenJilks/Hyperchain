use super::Block;
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
        return Some( Block::from_bytes(&bytes) );
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

    pub fn add(&self, block: &Block) -> std::io::Result<()>
    {
        if block.block_id > 1
        {
            let prev_or_none = self.block(block.block_id - 1);
            if prev_or_none.is_none() {
                return Ok(()); // FIXME: This should be an error
            }

            let prev = prev_or_none.unwrap();
            if !block.validate(Some( &prev )) {
                return Ok(()); // FIXME: This should be an error
            }
        }

        let bytes = block.as_bytes();
        if bytes.is_none() {
            return Ok(()); // FIXME: This should be an error
        }

        let mut file = File::create(self.blocks_path.join(block.block_id.to_string()))?;
        file.write(&bytes.unwrap())?;
        Ok(())
    }

}
