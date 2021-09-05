use super::{TransactionHeader, TransactionValidationResult};
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

impl TransactionHeader for Transfer
{

    fn validate(&self) -> Result<TransactionValidationResult, Box<dyn Error>>
    {
        if self.amount < 0.0 || self.fee < 0.0 {
            Ok(TransactionValidationResult::Negative)
        } else {
            Ok(TransactionValidationResult::Ok)
        }
    }

    fn update_wallet_status(&self, address: &Hash, mut status: WalletStatus,
                            is_from_address: bool, is_block_winner: bool)
        -> Option<WalletStatus>
    {
        if is_from_address
        {
            status.balance -= self.amount + self.fee;
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
    use super::super::Transaction;
    use crate::block::Block;
    use crate::chain::BlockChain;
    use crate::logger::{Logger, LoggerLevel};
    use crate::wallet::Wallet;
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::miner;

    use std::path::PathBuf;

    #[test]
    fn test_transfer()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::open_temp(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();

        let block = miner::mine_block(Block::new(&mut chain, &wallet).expect("Create block"));
        chain.add(&block, &mut logger).unwrap();

        {
            let transfer = Transaction::new(&wallet, Transfer::new(0, other.get_address(), 2.4, 0.2));
            transfer.hash().expect("Hash header");
            assert_eq!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = Transaction::new(&wallet, Transfer::new(1, other.get_address(), -1.6, 0.0));
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = Transaction::new(&wallet, Transfer::new(2, other.get_address(), 0.0, -0.0001));
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }
    }

}
