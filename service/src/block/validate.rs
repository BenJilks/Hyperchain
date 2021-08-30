use super::{Block, current_timestamp};
use super::target::{calculate_target, hash_from_target};
use crate::transaction::TransactionValidationResult;

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
        }
    }

}

pub trait BlockValidate
{
    fn is_next_block(&self, prev: &Block) -> Result<BlockValidationResult, Box<dyn Error>>;
    fn is_pow_valid(&self) -> Result<BlockValidationResult, Box<dyn Error>>;

    fn is_target_valid(&self, 
        start_sample: Option<&Block>, 
        end_sample: Option<&Block>) -> BlockValidationResult;

    fn is_valid(&self,
        start_sample: Option<&Block>, 
        end_sample: Option<&Block>) -> Result<BlockValidationResult, Box<dyn Error>>;
}

impl Block
{

    fn is_transaction_content_valid(&self) -> Result<BlockValidationResult, Box<dyn Error>>
    {
        for transaction in &self.transactions 
        {
            match transaction.is_valid()?
            {
                TransactionValidationResult::Ok => {},
                result => return Ok(BlockValidationResult::Transaction(result)),
            }
        }

        Ok(BlockValidationResult::Ok)
    }

}

impl BlockValidate for Block
{

    fn is_next_block(&self, prev: &Block) -> Result<BlockValidationResult, Box<dyn Error>>
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

    fn is_pow_valid(&self) -> Result<BlockValidationResult, Box<dyn Error>>
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

    fn is_target_valid(&self, 
                       start_sample: Option<&Block>, 
                       end_sample: Option<&Block>) -> BlockValidationResult
    {
        if self.target == calculate_target(start_sample, end_sample) {
            BlockValidationResult::Ok
        } else {
            BlockValidationResult::Target
        }
    }

    fn is_valid(&self,
                start_sample: Option<&Block>, 
                end_sample: Option<&Block>) -> Result<BlockValidationResult, Box<dyn Error>>
    {
        match self.is_pow_valid()?
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }
        match self.is_target_valid(start_sample, end_sample)
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }
        match self.is_transaction_content_valid()?
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }

        Ok(BlockValidationResult::Ok)
    }

}
