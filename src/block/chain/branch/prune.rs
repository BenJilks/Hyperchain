use super::super::prune::{Prune, PRUNE_LIMIT};
use super::Branch;
use crate::logger::{LoggerLevel, Logger};

use std::io::Write;

impl Branch
{

    fn merge_sub_branch_into_branch(&mut self, branch_id: i32)
    {
        let mut branch = self.sub_branches.remove(&branch_id).unwrap();
        for block_id in branch.bottom..=branch.top {
            self.blocks.insert(block_id, branch.blocks.remove(&block_id).unwrap());
        }
    }

    fn topmost_sub_branch(&self) -> Option<(i32, &Branch)>
    {
        let result = self.sub_branches
            .iter()
            .max_by(|(_, a), (_, b)| a.bottom.cmp(&b.bottom));
        
        match result
        {
            Some((id, branch)) => Some( (*id, branch) ),
            None => None,
        }
    }

    fn prune_this_branch(&mut self, logger: &mut Logger<impl Write>)
    {
        // Check if this branch needs to be replaced with a sub branch
        loop
        {
            let topmost_sub_branch_or_none = self.topmost_sub_branch();
            if topmost_sub_branch_or_none.is_none() {
                break;
            }

            // TODO: Write what we're doing here
            let (topmost_sub_branch_id, topmost_sub_branch) = topmost_sub_branch_or_none.unwrap();
            if topmost_sub_branch.top < self.top || topmost_sub_branch.top - self.top <= PRUNE_LIMIT {
                break;
            }

            logger.log(LoggerLevel::Warning, 
                &format!("Prune this branch and replace with sub branch {}", topmost_sub_branch_id));
            self.merge_sub_branch_into_branch(topmost_sub_branch_id);
        }
    }

    fn prune_smaller_sub_branches(&mut self, logger: &mut Logger<impl Write>)
    {
        let mut sub_branches_to_prune = Vec::<i32>::new();
        for (sub_branch_id, sub_branch) in &self.sub_branches
        {
            let top_id = sub_branch.top().block_id;
            if top_id < self.top && self.top - top_id > PRUNE_LIMIT {
                sub_branches_to_prune.push(*sub_branch_id);
            }
        }

        for sub_branch_id in sub_branches_to_prune 
        {
            logger.log(LoggerLevel::Warning, 
                &format!("Prune sub branch {}", sub_branch_id));
            self.sub_branches.remove(&sub_branch_id);
        }
    }

}

impl Prune for Branch
{

    fn prune(&mut self, logger: &mut Logger<impl Write>)
    {
        for (_, sub_branch) in &mut self.sub_branches {
            sub_branch.prune(logger);
        }

        self.prune_this_branch(logger);
        self.prune_smaller_sub_branches(logger);
    }

}

