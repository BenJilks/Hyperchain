use super::Branch;
use crate::block::Block;

#[derive(PartialEq, Debug)]
pub enum CanAddResult
{
    Yes,
    InSubBranch(i32),
    Duplicate,
    Invalid,
}

pub trait BranchAdd
{
    fn can_add(&self, block: &Block) -> CanAddResult;
    fn try_add(&mut self, block: &Block) -> CanAddResult;
}

impl BranchAdd for Branch
{

    fn can_add(&self, block: &Block) -> CanAddResult
    {
        // Check duplicate
        let existing_block_or_none = self.blocks.get(&block.block_id);
        if existing_block_or_none.is_some()
        {
            let existing_block = existing_block_or_none.unwrap();
            if existing_block == block {
                return CanAddResult::Duplicate;
            } else {
                return CanAddResult::Invalid;
            }
        }

        // Check this is the next block in the chain
        if block.block_id == self.top + 1
        {
            let last = self.blocks.get(&(block.block_id - 1));
            if block.is_next_block(last.unwrap()).is_ok() {
                return CanAddResult::Yes;
            }
        }
        
        // Check this is the previus block in the chain
        if block.block_id == self.bottom - 1
        {
            let next = self.blocks.get(&(block.block_id + 1));
            if next.unwrap().is_next_block(block).is_ok() {
                return CanAddResult::Yes;
            }
        }
       
        // Check sub branches
        for (id, sub_branch) in &self.sub_branches 
        {
            match sub_branch.can_add(block)
            {
                CanAddResult::Invalid => 
                    {},

                CanAddResult::Duplicate => 
                    return CanAddResult::Duplicate,

                CanAddResult::Yes | CanAddResult::InSubBranch(_) => 
                    return CanAddResult::InSubBranch(*id),
            }
        }

        CanAddResult::Invalid
    }

    fn try_add(&mut self, block: &Block) -> CanAddResult
    {
        match self.can_add(block)
        {
            CanAddResult::Yes =>
            {
                self.top = std::cmp::max(self.top, block.block_id);
                self.bottom = std::cmp::min(self.bottom, block.block_id);
                self.blocks.insert(block.block_id, block.clone());
                CanAddResult::Yes
            },

            CanAddResult::InSubBranch(id) =>
            {
                let branch = &mut self.sub_branches.get_mut(&id);
                assert_eq!(branch.is_some(), true);
                branch.as_mut().unwrap().try_add(block)
            },

            err => err,
        }
    }

}

