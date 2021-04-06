mod branch;
pub use branch::BlockChainBranch;
use super::Block;
use crate::error::Error;

use std::fs;
use std::path::PathBuf;
use rand::RngCore;

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

    // If any branches are more then 10 blocks behind the longest, it's deleted
    pub fn prune_branches(&mut self)
    {
        let longest_branch_top = self.longest_branch().top_index;
        if longest_branch_top <= 10 {
            return;
        }

        let mut branches_to_remove = Vec::<BlockChainBranch>::new();
        for branch in &self.branches
        {
            if branch.top_index < longest_branch_top - 10 {
                branches_to_remove.push(branch.clone());
            }
        }

        for branch in &branches_to_remove
        {
            let index = self.branches.iter().position(|x| *x == *branch).unwrap();
            self.branches.remove(index);
            std::fs::remove_dir_all(&branch.path).unwrap();
        }

        if branches_to_remove.len() > 0 {
            println!("Pruned {} branches", branches_to_remove.len());
        }
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

        // If no branches exist, add a new one
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

    pub fn top_id(&mut self) -> u64
    {
        let top = self.top();
        if top.is_some() {
            top.unwrap().block_id
        } else {
            0
        }
    }

    fn branch(&mut self, old_branch: &BlockChainBranch, block: &Block) -> Result<(), Error>
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

    pub fn add(&mut self, block: &Block) -> Result<(), Error>
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

            if block.block_id > branch.top_index {
                continue;
            }

            if &branch.block(block.block_id).unwrap() == block {
                return Err(Error::DuplicateBlock);
            }

            if block.validate(&branch).is_ok()
            {
                valid_to_branch_from = Some( branch.clone() );
                break;
            }
        }

        if valid_to_branch_from.is_none() {
            return Err(Error::NoValidBranches)
        }

        self.branch(&valid_to_branch_from.unwrap(), block)?;
        Ok(())
    }

}
