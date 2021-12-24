use super::{Input, TransactionContent, TransactionValidationResult};
use crate::wallet::WalletStatus;
use crate::error::ErrorMessage;
use crate::config::Hash;

use serde::{Serialize, Deserialize};
use std::error::Error;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Output
{
    pub to: Hash,
    pub amount: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Transfer
{
    pub id: u32,
    pub outputs: Vec<Output>,
    pub fee: f32,
}

pub struct TransferBuilder
{
    id: u32,
    fee: f32,
    outputs: Vec<Output>,
}

impl TransferBuilder
{

    pub fn new(id: u32, fee: f32) -> Self
    {
        Self
        {
            id,
            fee,
            outputs: Vec::new(),
        }
    }

    pub fn add_output(mut self, to: Hash, amount: f32) -> Self
    {
        self.outputs.push(Output
        {
            to,
            amount,
        });
        self
    }

    pub fn build(self) -> Transfer
    {
        Transfer::new(self.id, self.outputs, self.fee)
    }

}

impl Transfer
{

    pub fn new(id: u32, outputs: Vec<Output>, fee: f32) -> Self
    {
        Self 
        { 
            id,
            outputs,
            fee,
        }
    }

}

impl TransactionContent for Transfer
{
    
    fn get_fee(&self) -> f32
    {
        self.fee
    }

    fn validate(&self, inputs: &Vec<Input>) 
        -> Result<TransactionValidationResult, Box<dyn Error>>
    {
        let total_input = inputs.iter().fold(0.0, |acc, x| acc + x.amount);
        let total_output = self.outputs.iter().fold(0.0, |acc, x| acc + x.amount) + self.fee;
        if total_input != total_output {
            return Ok(TransactionValidationResult::Negative);
        }

        // FIXME: Check each output > 0
        if total_output < 0.0 || self.fee < 0.0 {
            return Ok(TransactionValidationResult::Negative);
        }
        
        Ok(TransactionValidationResult::Ok)
    }

    fn update_wallet_status(&self, address: &Hash, mut status: WalletStatus,
                            from_amount: f32, is_block_winner: bool)
        -> Result<WalletStatus, Box<dyn Error>>
    {
        if from_amount > 0.0
        {
            status.balance -= from_amount;
            if self.id <= status.max_id 
            {
                return Err(ErrorMessage::new(
                    &format!("Id is not incremental ({} -> {})",
                        status.max_id, self.id)));
            }
            status.max_id = self.id;
        }

        for output in &self.outputs
        {
            if &output.to == address {
                status.balance += output.amount;
            }
        }

        if is_block_winner {
            status.balance += self.fee;
        }

        Ok(status)
    }

    fn get_to_addresses(&self) -> Vec<Hash>
    {
        self.outputs
            .iter()
            .map(|x| x.to)
            .collect()
    }

    fn get_id(&self) -> u32
    {
        self.id
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

    #[test]
    fn test_transfer()
    {
        let _ = pretty_env_logger::try_init();

        let mut chain = BlockChain::open_temp();
        let wallet = PrivateWallet::open_temp(0).unwrap();
        let other = PrivateWallet::open_temp(1).unwrap();

        let block = miner::mine_block(Block::new_blank(&mut chain, &wallet).expect("Create block"));
        chain.add(&block).unwrap();

        {
            let transfer = 
                TransactionBuilder::new(
                    TransferBuilder::new(0, 0.2)
                        .add_output(other.get_address(), 2.4)
                        .build())
                    .add_input(&wallet, 2.4 + 0.2)
                    .build().unwrap();
            transfer.hash().expect("Hash header");
            assert_eq!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = 
                TransactionBuilder::new(
                    TransferBuilder::new(1, 0.0)
                        .add_output(other.get_address(), -1.6)
                        .build())
                    .add_input(&wallet, -1.6)
                    .build().unwrap();
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = 
                TransactionBuilder::new(
                    TransferBuilder::new(2, -0.0001)
                        .add_output(other.get_address(), 0.0)
                        .build())
                    .add_input(&wallet, -0.0001)
                    .build().unwrap();
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = 
                TransactionBuilder::new(
                    TransferBuilder::new(2, 1.0)
                        .add_output(other.get_address(), 5.0)
                        .add_output(wallet.get_address(), 5.0)
                        .build())
                    .add_input(&wallet, 5.0)
                    .add_input(&other, 6.0)
                    .build().unwrap();
            assert_eq!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }

        {
            let transfer = 
                TransactionBuilder::new(
                    TransferBuilder::new(2, 1.0)
                        .add_output(other.get_address(), 5.0)
                        .add_output(wallet.get_address(), 5.0)
                        .build())
                    .add_input(&wallet, 5.0)
                    .add_input(&other, 5.0)
                    .build().unwrap();
            assert_ne!(transfer.validate_content().unwrap(), TransactionValidationResult::Ok);
        }
    }

}

