use super::{Block, Hash, current_timestamp};
use super::transactions::BlockTransactions;
use super::target::{calculate_target, hash_from_target};
use crate::transaction::TransactionValidationResult;
use crate::chain::BlockChain;
use crate::wallet::get_status_for_address;

use rsa::BigUint;
use std::error::Error;

#[derive(Debug, PartialEq)]
pub enum BlockValidationResult
{
    Ok,
    NotNextBlock,
    PrevHash,
    Timestamp,
    POW,
    Target,
    Transaction(TransactionValidationResult),
    Balance(Hash),
}

impl std::fmt::Display for BlockValidationResult
{

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self
        {
            BlockValidationResult::Ok => write!(f, "Ok"),
            BlockValidationResult::NotNextBlock => write!(f, "Not the next block in the chain"),
            BlockValidationResult::PrevHash => write!(f, "Previous hash does not match"),
            BlockValidationResult::Timestamp => write!(f, "Timestamp not in a valid range"),
            BlockValidationResult::POW => write!(f, "No valid proof or work"),
            BlockValidationResult::Target => write!(f, "Incorrect target value"),
            BlockValidationResult::Transaction(result) => write!(f, "{}", result),
            BlockValidationResult::Balance(_) => write!(f, "Insufficient balance"),
        }
    }

}

pub trait BlockValidate
{
    fn validate_next(&self, prev: &Block) 
        -> Result<BlockValidationResult, Box<dyn Error>>;

    fn validate_pow(&self) 
        -> Result<BlockValidationResult, Box<dyn Error>>;

    fn validate_target(&self, 
        start_sample: Option<Block>, 
        end_sample: Option<Block>) -> BlockValidationResult;

    fn validate_content(&self,
        start_sample: Option<Block>, 
        end_sample: Option<Block>) 
        -> Result<BlockValidationResult, Box<dyn Error>>;

    fn validate(&self, chain: &BlockChain) 
        -> Result<BlockValidationResult, Box<dyn Error>>;
}

fn validate_transactions(block: &Block) -> Result<BlockValidationResult, Box<dyn Error>>
{
    for transaction in &block.transactions 
    {
        match transaction.validate_content()?
        {
            TransactionValidationResult::Ok => {},
            result => return Ok(BlockValidationResult::Transaction(result)),
        }
    }

    Ok(BlockValidationResult::Ok)
}

impl BlockValidate for Block
{

    fn validate_next(&self, prev: &Block) -> Result<BlockValidationResult, Box<dyn Error>>
    {
        if self.block_id > 0
        {
            if self.block_id != prev.block_id + 1 {
                return Ok(BlockValidationResult::NotNextBlock);
            }

            if self.prev_hash != prev.hash()? {
                return Ok(BlockValidationResult::PrevHash);
            }

            let now = current_timestamp();
            if self.timestamp < prev.timestamp || self.timestamp > now {
                return Ok(BlockValidationResult::Timestamp);
            }
        }

        Ok(BlockValidationResult::Ok)
    }

    fn validate_pow(&self) -> Result<BlockValidationResult, Box<dyn Error>>
    {
        let hash = self.hash()?;
        let hash_num = BigUint::from_bytes_be(&hash);
        let target_num = BigUint::from_bytes_be(&hash_from_target(&self.target));
        if hash_num < target_num {
            Ok(BlockValidationResult::Ok)
        } else {
            Ok(BlockValidationResult::POW)
        }
    }

    fn validate_target(&self, 
                       start_sample: Option<Block>, 
                       end_sample: Option<Block>) -> BlockValidationResult
    {
        if self.target == calculate_target(start_sample, end_sample) {
            BlockValidationResult::Ok
        } else {
            BlockValidationResult::Target
        }
    }

    fn validate_content(&self,
                start_sample: Option<Block>, 
                end_sample: Option<Block>) -> Result<BlockValidationResult, Box<dyn Error>>
    {
        match self.validate_pow()?
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }
        match self.validate_target(start_sample, end_sample)
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }
        match validate_transactions(self)?
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }

        Ok(BlockValidationResult::Ok)
    }

    fn validate(&self, chain: &BlockChain) 
        -> Result<BlockValidationResult, Box<dyn Error>>
    {
        let (sample_start, sample_end) = 
            if self.block_id == 0 {
                (None, None)
            } else {
                chain.take_sample_at(self.block_id - 1)
            };

        if sample_end.is_some()
        {
            match self.validate_next(sample_end.as_ref().unwrap())?
            {
                BlockValidationResult::Ok => {},
                err => return Ok(err),
            }
        }

        match self.validate_content(sample_start, sample_end)?
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }

        for address in self.get_addresses_used() 
        {
            let mut status = get_status_for_address(chain, &address);
            self.update_wallet_status(&address, &mut status);

            if status.balance < 0.0 {
                return Ok(BlockValidationResult::Balance(address));
            }
        }

        Ok(BlockValidationResult::Ok)
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::block::transactions::BlockTransactions;
    use crate::transaction::Transaction;
    use crate::chain::BlockChain;
    use crate::logger::{Logger, LoggerLevel};
    use crate::wallet::{WalletStatus, Wallet};
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::miner;
    use std::path::PathBuf;

    #[test]
    fn test_block_verify()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();
        let chain = BlockChain::new(&mut logger);

        let mut block = Block::new(&chain, &wallet).expect("Can create block");
        let transaction = Transaction::for_chain(&chain, &wallet, other.get_address(), 4.0, 1.0)
            .expect("Create transaction");
        block.add_transaction(transaction);

        assert_ne!(block.validate_pow().unwrap(), BlockValidationResult::Ok);
        assert_eq!(block.validate_target(None, None), BlockValidationResult::Ok);
        assert_ne!(block.validate_content(None, None).unwrap(), BlockValidationResult::Ok);

        block = miner::mine_block(block);
        assert_eq!(block.validate_pow().unwrap(), BlockValidationResult::Ok);
        assert_eq!(block.validate_content(None, None).unwrap(), BlockValidationResult::Ok);

        {
            let mut wallet_status = WalletStatus::default();
            block.update_wallet_status(&wallet.get_address(), &mut wallet_status);
            assert_eq!(wallet_status.balance, block.calculate_reward() - 4.0);
            assert_eq!(wallet_status.max_id, 1);
        }

        {
            let mut wallet_status = WalletStatus::default();
            block.update_wallet_status(&other.get_address(), &mut wallet_status);
            assert_eq!(wallet_status.balance, 4.0);
            assert_eq!(wallet_status.max_id, 0);
        }

        let addresses_used = block.get_addresses_used();
        assert_eq!(addresses_used.len(), 2);
        assert_eq!(addresses_used.contains(&wallet.get_address()), true);
        assert_eq!(addresses_used.contains(&other.get_address()), true);
    }

}

