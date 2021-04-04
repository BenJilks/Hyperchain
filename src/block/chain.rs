use super::{Block, Page, DataFormat};
use crate::wallet::{Wallet, PublicWallet};
use std::fs::{self, File};
use std::io::{Write, Read};
use std::path::PathBuf;
use rand::RngCore;

#[derive(Debug, Clone)]
pub struct BlockChainBranch
{
    path: PathBuf,
    blocks_path: PathBuf,

    top_index: u64,
}

fn io_error<T>(result: std::io::Result<T>) -> Result<T, String>
{
    if result.is_ok() {
        Ok(result.ok().unwrap())
    } else {
        Err(result.err().unwrap().to_string())
    }
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
        fs::create_dir_all(&blocks_path).unwrap();

        let top_index = Self::find_top_index(&blocks_path);
        Self
        {
            path,
            blocks_path,
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

    pub fn top(&self) -> Option<Block>
    {
        if self.top_index == 0 {
            None
        } else {
            self.block(self.top_index)
        }
    }

    pub fn add(&mut self, block: &Block) -> Result<(), String>
    {
        if block.block_id != self.top_index + 1 {
            return Err("Not next block".to_owned());
        }

        if block.block_id > 1 {
            block.validate(self)?
        }

        if !block.validate_pow() {
            return Err("Invalid block (POW)".to_owned());
        }

        let bytes = block.as_bytes()?;
        let block_path = self.blocks_path.join(block.block_id.to_string());
        let mut file = io_error(File::create(block_path))?;
        io_error(file.write(&bytes))?;

        //for page in &block.pages {
        //    self.apply_page(page)?;
        //}

        self.top_index += 1;
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

pub struct BlockChain
{
    path: PathBuf,
    branches: Vec<BlockChainBranch>,
}

impl BlockChain
{

    pub fn new(path: PathBuf) -> Self
    {
        fs::create_dir_all(&path).unwrap();

        let mut branches = Vec::<BlockChainBranch>::new();
        for file_or_error in std::fs::read_dir(&path).unwrap()
        {
            if file_or_error.is_err() {
                continue;
            }

            let file = file_or_error.unwrap();
            if !file.file_type().unwrap().is_dir() {
                continue;
            }

            branches.push(BlockChainBranch::new(file.path()));
        }

        Self
        {
            path,
            branches,
        }
    }

    pub fn block(&self, id: u64) -> Vec<Block>
    {
        let mut blocks = Vec::<Block>::new();
        for branch in &self.branches
        {
            let block = branch.block(id);
            if block.is_some() {
                blocks.push(block.unwrap());
            }
        }

        blocks
    }

    pub fn longest_branch(&mut self) -> &mut BlockChainBranch
    {
        let mut max_branch_index = None;
        let mut max_top = 0u64;
        for i in 0..self.branches.len()
        {
            let branch = &self.branches[i];
            if branch.top_index >= max_top 
            {
                max_top = branch.top_index;
                max_branch_index = Some( i );
            }
        }

        if max_branch_index.is_none() 
        {
            self.branches.push(BlockChainBranch::new(self.path.join("master")));
            max_branch_index = Some( 0 );
        }

        &mut self.branches[max_branch_index.unwrap()]
    }

    pub fn top(&mut self) -> Option<Block>
    {
        self.longest_branch().top()
    }

    fn branch(&mut self, old_branch: &BlockChainBranch, block: &Block) -> Result<(), String>
    {
        let mut new_branch_id = [0u8; 5];
        rand::thread_rng().fill_bytes(&mut new_branch_id);
        
        let new_branch_path = self.path.join(base_62::encode(&new_branch_id));
        let mut branch = BlockChainBranch::new(new_branch_path);
        for i in 1..=(block.block_id - 1) {
            branch.add(&old_branch.block(i).unwrap())?;
        }
        branch.add(block)?;

        self.branches.push(branch);
        Ok(())
    }

    pub fn add(&mut self, block: &Block) -> Result<(),String>
    {
        let mut valid_to_branch_from = None;
        for branch in &mut self.branches
        {
            if block.block_id == branch.top_index + 1
            {
                if branch.add(block).is_ok() {
                    return Ok(());
                }
            }

            if block.validate(&branch).is_ok()
            {
                valid_to_branch_from = Some( branch.clone() );
                break;
            }
        }

        if valid_to_branch_from.is_none() {
            return Err("No valid branches".to_owned())
        }

        self.branch(&valid_to_branch_from.unwrap(), block)?;
        Ok(())
    }

    /*
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
    */

    /*
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
    */

}
