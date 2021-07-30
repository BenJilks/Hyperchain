use super::{Block, current_timestamp, calculate_target};
use crate::error::Error;

use rsa::BigUint;

pub trait BlockValidate
{
    fn is_next_block(&self, prev: &Block) -> Result<(), Error>;
    fn is_pow_valid(&self) -> bool;
    fn is_target_valid(&self) -> bool;
    fn is_valid(&self) -> bool;
}

impl Block
{

    fn is_transaction_content_valid(&self) -> bool
    {
        for transaction in &self.transactions
        {
            if !transaction.is_valid() {
                return false;
            }
        }

        return true;
    }

}

impl BlockValidate for Block
{

    fn is_next_block(&self, prev: &Block) -> Result<(), Error>
    {
        if self.block_id > 1
        {
            if self.block_id != prev.block_id + 1 {
                return Err(Error::NotNextBlock);
            }

            if self.prev_hash != prev.hash()? {
                return Err(Error::PrevInvalidHash);
            }

            let now = current_timestamp();
            if self.timestamp < prev.timestamp || self.timestamp > now {
                return Err(Error::InvalidTimestamp);
            }
        }

        Ok(())
    }

    fn is_pow_valid(&self) -> bool
    {
        let hash_or_none = self.hash();
        if hash_or_none.is_err()
        {
            println!("Faild to hash: {}", hash_or_none.err().unwrap().to_string());
            return false;
        }

        // Validate POW
        let hash = hash_or_none.ok().unwrap();
        let hash_num = BigUint::from_bytes_le(&hash);
        let target_num = BigUint::from_bytes_le(&self.target);
        if hash_num > target_num {
            return false;
        }

        true
    }

    fn is_target_valid(&self) -> bool
    {
        self.target == calculate_target()
    }

    fn is_valid(&self) -> bool
    {
        self.is_pow_valid() 
            && self.is_target_valid()
            && self.is_transaction_content_valid()
    }

}

