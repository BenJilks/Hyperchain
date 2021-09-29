use super::{Block, Hash, current_timestamp};
use super::target::{calculate_target, hash_from_target};
use super::transactions::merkle_root_for_transactions;
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
    MerkleRoot,
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
            BlockValidationResult::MerkleRoot => write!(f, "Incorrect merkle root"),
            BlockValidationResult::Transaction(result) => write!(f, "{}", result),
            BlockValidationResult::Balance(_) => write!(f, "Insufficient balance"),
        }
    }

}

impl Block
{

    fn validate_transactions(&self) 
        -> Result<BlockValidationResult, Box<dyn Error>>
    {
        let merkle_root = merkle_root_for_transactions(&self.transfers, &self.pages)?;
        if merkle_root != self.header.transaction_merkle_root {
            return Ok(BlockValidationResult::MerkleRoot);
        }

        for transfer in &self.transfers
        {
            match transfer.validate_content()?
            {
                TransactionValidationResult::Ok => {},
                result => return Ok(BlockValidationResult::Transaction(result)),
            }
        }

        Ok(BlockValidationResult::Ok)
    }

    pub fn validate_next(&self, prev: &Block) 
        -> Result<BlockValidationResult, Box<dyn Error>>
    {
        if self.header.block_id > 0
        {
            if self.header.block_id != prev.header.block_id + 1 {
                return Ok(BlockValidationResult::NotNextBlock);
            }

            if self.header.prev_hash != prev.hash()? {
                return Ok(BlockValidationResult::PrevHash);
            }

            let now = current_timestamp();
            if self.header.timestamp < prev.header.timestamp || self.header.timestamp > now {
                return Ok(BlockValidationResult::Timestamp);
            }
        }

        Ok(BlockValidationResult::Ok)
    }

    pub fn validate_pow(&self) 
        -> Result<BlockValidationResult, Box<dyn Error>>
    {
        let hash = self.hash()?;
        let hash_num = BigUint::from_bytes_be(&hash);
        let target_num = BigUint::from_bytes_be(&hash_from_target(&self.header.target));
        if hash_num < target_num {
            Ok(BlockValidationResult::Ok)
        } else {
            Ok(BlockValidationResult::POW)
        }
    }

    pub fn validate_target(&self, 
                           start_sample: Option<Block>, 
                           end_sample: Option<Block>) 
        -> BlockValidationResult
    {
        if self.header.target == calculate_target(start_sample, end_sample) {
            BlockValidationResult::Ok
        } else {
            BlockValidationResult::Target
        }
    }

    pub fn validate_content(&self,
                            start_sample: Option<Block>, 
                            end_sample: Option<Block>) 
        -> Result<BlockValidationResult, Box<dyn Error>>
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
        match self.validate_transactions()?
        {
            BlockValidationResult::Ok => {},
            err => return Ok(err),
        }

        Ok(BlockValidationResult::Ok)
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::super::builder::BlockBuilder;
    use crate::transaction::transfer::TransferBuilder;
    use crate::transaction::builder::TransactionBuilder;
    use crate::chain::BlockChain;
    use crate::wallet::{WalletStatus, Wallet};
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::miner;

    #[test]
    fn test_block_verify()
    {
        let wallet = PrivateWallet::open_temp(0).unwrap();
        let other = PrivateWallet::open_temp(1).unwrap();
        let mut chain = BlockChain::open_temp();

        let transaction = 
            TransactionBuilder::new(
                TransferBuilder::new(1, 1.0)
                    .add_output(other.get_address(), 4.0)
                    .build())
                .add_input(&wallet, 5.0)
                .build().unwrap();
        let mut block = BlockBuilder::new(&wallet)
            .add_transfer(transaction)
            .build(&mut chain)
            .expect("Can create block");

        assert_ne!(block.validate_pow().unwrap(), BlockValidationResult::Ok);
        assert_eq!(block.validate_target(None, None), BlockValidationResult::Ok);
        assert_ne!(block.validate_content(None, None).unwrap(), BlockValidationResult::Ok);

        block = miner::mine_block(block);
        assert_eq!(block.validate_pow().unwrap(), BlockValidationResult::Ok);
        assert_eq!(block.validate_content(None, None).unwrap(), BlockValidationResult::Ok);

        {
            let mut wallet_status = WalletStatus::default();
            wallet_status = block.update_wallet_status(&wallet.get_address(), wallet_status).unwrap();
            assert_eq!(wallet_status.balance, block.calculate_reward() - 4.0);
            assert_eq!(wallet_status.max_id, 1);
        }

        {
            let mut wallet_status = WalletStatus::default();
            wallet_status = block.update_wallet_status(&other.get_address(), wallet_status).unwrap();
            assert_eq!(wallet_status.balance, 4.0);
            assert_eq!(wallet_status.max_id, 0);
        }

        let addresses_used = block.get_addresses_used();
        assert_eq!(addresses_used.len(), 2);
        assert_eq!(addresses_used.contains(&wallet.get_address()), true);
        assert_eq!(addresses_used.contains(&other.get_address()), true);
    }

}

