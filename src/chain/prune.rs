use super::BlockChain;
use crate::logger::{LoggerLevel, Logger};

use std::io::Write;

pub const PRUNE_LIMIT: u64 = 50;

pub trait Prune
{
    
    fn prune(&mut self, logger: &mut Logger<impl Write>);

}

impl Prune for BlockChain
{

    fn prune(&mut self, logger: &mut Logger<impl Write>)
    {
        let current_branch = self.current_branch();
        if current_branch.is_none() {
            return;
        }

        let current_top_id = current_branch.unwrap().top().block_id;
        let mut branches_to_prune = Vec::<i32>::new();
        for (branch_id, branch) in &mut self.branches
        {
            let top_id = branch.top().block_id;
            if current_top_id - top_id > PRUNE_LIMIT {
                branches_to_prune.push(*branch_id);
            } else {
                branch.prune(logger);
            }
        }

        for branch_id in branches_to_prune 
        {
            logger.log(LoggerLevel::Warning, 
                &format!("Prune branch {}", branch_id));
            self.branches.remove(&branch_id);
        }
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::miner;
    use crate::wallet::PrivateWallet;
    use crate::block::Block;

    use std::path::PathBuf;

    #[test]
    fn test_prune()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::new(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();

        let mut mine_block = || 
        {
            let block = Block::new(chain.current_branch(), &wallet).unwrap();
            chain.add(&miner::mine_block(block), &mut logger);
        };

        mine_block();
    }

}

