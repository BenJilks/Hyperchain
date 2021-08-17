use super::{Block, current_timestamp};
use super::target::{calculate_target, hash_from_target};
use crate::error::Error;

use rsa::BigUint;

pub trait BlockValidate
{
    fn is_next_block(&self, prev: &Block) -> Result<(), Error>;
    fn is_pow_valid(&self) -> bool;

    fn is_target_valid(&self, 
        start_sample: Option<&Block>, 
        end_sample: Option<&Block>) -> bool;

    fn is_valid(&self,
        start_sample: Option<&Block>, 
        end_sample: Option<&Block>) -> bool;
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
        let hash_num = BigUint::from_bytes_be(&hash);
        let target_num = BigUint::from_bytes_be(&hash_from_target(&self.target));
        return hash_num < target_num;
    }

    fn is_target_valid(&self, 
                       start_sample: Option<&Block>, 
                       end_sample: Option<&Block>) -> bool
    {
        self.target == calculate_target(start_sample, end_sample)
    }

    fn is_valid(&self,
                start_sample: Option<&Block>, 
                end_sample: Option<&Block>) -> bool
    {
        self.is_pow_valid() 
            && self.is_target_valid(start_sample, end_sample)
            && self.is_transaction_content_valid()
    }

}

