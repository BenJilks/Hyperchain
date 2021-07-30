use super::Branch;
use crate::block::validate::BlockValidate;

#[derive(PartialEq)]
enum MergeType
{
    Extends,
    OtherIsSubBranch,
    WeAreSubChain,
    MergeSubBranch(i32),
    NoMerge,
}

pub trait BranchMerge
{
    fn can_merge(&self, other: &Branch) -> bool;
    fn merge(&mut self, other: Branch);
}

impl Branch
{

    fn get_merge_type(&self, other: &Branch) -> MergeType
    {
        // Extends the top
        if other.bottom == self.top + 1
        {
            let our_top = &self.blocks[&self.top];
            let other_bottom = &other.blocks[&other.bottom];
            if other_bottom.is_next_block(our_top).is_ok() {
                return MergeType::Extends;
            }
        }

        // Extends the bottom
        if other.top == self.bottom - 1
        {
            let other_top = &other.blocks[&other.top];
            let our_bottom = &self.blocks[&self.bottom];
            if our_bottom.is_next_block(other_top).is_ok() {
                return MergeType::Extends;
            }
        }

        // Is other a sub-chain of us
        if other.bottom > self.bottom && other.bottom < self.top 
        {
            let root = &self.blocks[&(other.bottom - 1)];
            let next = &other.blocks[&other.bottom];
            if next.is_next_block(root).is_ok() {
                return MergeType::OtherIsSubBranch;
            }
        }

        // Are we a sub-chain of other
        if self.bottom > other.bottom && self.bottom < other.top 
        {
            let root = &other.blocks[&(self.bottom - 1)];
            let next = &self.blocks[&self.bottom];
            if next.is_next_block(root).is_ok() {
                return MergeType::WeAreSubChain;
            }
        }

        // Check sub branches
        for (id, sub_branch) in &self.sub_branches
        {
            match sub_branch.get_merge_type(other)
            {
                MergeType::NoMerge => 
                    {},

                _ => 
                    return MergeType::MergeSubBranch(*id),
            }
        }

        MergeType::NoMerge
    }

    fn add_sub_branch(&mut self, sub_branch: Branch)
    {
        let mut branch_id = rand::random::<i32>();
        while self.sub_branches.contains_key(&branch_id) {
            branch_id = rand::random::<i32>();
        }
        self.sub_branches.insert(branch_id, sub_branch);
    }

}

impl BranchMerge for Branch
{

    fn can_merge(&self, other: &Branch) -> bool
    {
        self.get_merge_type(other) != MergeType::NoMerge
    }

    fn merge(&mut self, mut other: Branch)
    {
        match self.get_merge_type(&other)
        {
            MergeType::Extends =>
            {
                for i in other.bottom..=other.top {
                    self.blocks.insert(i, other.blocks[&i].clone());
                }

                self.top = std::cmp::max(self.top, other.top);
                self.bottom = std::cmp::min(self.bottom, other.bottom);
            },

            MergeType::OtherIsSubBranch =>
            {
                self.add_sub_branch(other);
            },

            MergeType::WeAreSubChain =>
            {
                std::mem::swap(self, &mut other);
                self.add_sub_branch(other);
            },

            MergeType::MergeSubBranch(id) =>
            {
                let sub_branch = &mut self.sub_branches.get_mut(&id).unwrap();
                sub_branch.merge(other);
            },

            MergeType::NoMerge =>
            {
                panic!();
            },
        }
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::super::add::BranchAdd;
    use super::super::tests::create_blocks;

    #[test]
    fn test_merge()
    {
        let chain = create_blocks(5, 1, None);
        
        let mut branch_a = Branch::new(chain[0].clone());
        branch_a.try_add(&chain[1]);
        branch_a.try_add(&chain[2]);

        let mut branch_b = Branch::new(chain[3].clone());
        branch_b.try_add(&chain[4]);

        assert_eq!(branch_a.can_merge(&branch_b), true);
        assert_eq!(branch_b.can_merge(&branch_a), true);
    }

}

