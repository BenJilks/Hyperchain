use super::{Input, TransactionContent, TransactionValidationResult};
use crate::wallet::WalletStatus;
use crate::config::Hash;

use serde::{Serialize, Deserialize};
use std::error::Error;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Transfer
{
    pub id: u32,
    pub to: Hash,
    pub amount: f32,
    pub fee: f32,
}

impl Transfer
{

    pub fn new(id: u32, to: Hash, amount: f32, fee: f32) -> Self
    {
        Self 
        { 
            id,
            to,
            amount,
            fee,
        }
    }

}

impl TransactionContent for Transfer
{

    fn validate(&self, inputs: &Vec<Input>) 
        -> Result<TransactionValidationResult, Box<dyn Error>>
    {
        let total_input = inputs.iter().fold(0.0, |acc, x| acc + x.amount);
        let total_output = self.amount + self.fee;
        if total_input != total_output {
            return Ok(TransactionValidationResult::Negative);
        }

        if self.amount < 0.0 || self.fee < 0.0 {
            return Ok(TransactionValidationResult::Negative);
        }
        
        Ok(TransactionValidationResult::Ok)
    }

    fn update_wallet_status(&self, address: &Hash, mut status: WalletStatus,
                            from_amount: f32, is_block_winner: bool)
        -> Option<WalletStatus>
    {
        if from_amount > 0.0
        {
            status.balance -= from_amount;
            if self.id <= status.max_id {
                return None;
            }
            status.max_id = self.id;
        }

        if &self.to == address {
            status.balance += self.amount;
        }

        if is_block_winner {
            status.balance += self.fee;
        }

        Some(status)
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::super::builder::TransactionBuilder;
    use crate::block::Block;
    use crate::chain::BlockChain;
    use crate::wallet::Wallet;
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::miner;

    use std::path::PathBuf;

    #[test]
    fn test_transfer()
    {
        let _ = pretty_env_logger::try_init();

        let mut chain = BlockChain::open_temp();
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();

        let block = miner::mine_block(Block::new_blank(&mut chain, &wallet).expect("Create block"));
        chain.add(&block).unwrap();

        {
            let transfer = TransactionBuilder::new(Transfer::new(0, other.get_address(), 2.4, 0.2))
                .add_input(&wallet, 2.4 + 0.2)
                .build().unwrap();
            transfer.hash().expect("Hash header");
            assert_eq!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = TransactionBuilder::new(Transfer::new(1, other.get_address(), -1.6, 0.0))
                .add_input(&wallet, -1.6)
                .build().unwrap();
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = TransactionBuilder::new(Transfer::new(2, other.get_address(), 0.0, -0.0001))
                .add_input(&wallet, -0.0001)
                .build().unwrap();
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = TransactionBuilder::new(Transfer::new(2, other.get_address(), 10.0, 1.0))
                .add_input(&wallet, 5.0)
                .add_input(&other, 6.0)
                .build().unwrap();
            assert_eq!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = TransactionBuilder::new(Transfer::new(2, other.get_address(), 10.0, 1.0))
                .add_input(&wallet, 5.0)
                .add_input(&other, 5.0)
                .build().unwrap();
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }
    }

}

