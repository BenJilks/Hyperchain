use super::{Input, TransactionContent, TransactionValidationResult};
use crate::wallet::WalletStatus;
use crate::data_store::data_unit::DataUnit;
use crate::error::ErrorMessage;
use crate::config::{Hash, PAGE_CHUNK_SIZE};

use serde::{Serialize, Deserialize};
use std::error::Error;

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Page
{
    pub id: u32,
    pub site: Hash,

    pub data_hashes: Vec<Hash>,
    pub data_length: u32,
    pub fee: f32,
}

impl Page
{

    pub fn new(id: u32, site: Hash, 
               data_hashes: Vec<Hash>, data_length: u32, 
               fee: f32) -> Self
    {
        Page
        {
            id,
            site,

            data_hashes,
            data_length,
            fee,
        }
    }

    pub fn new_from_data(id: u32, site: Hash, data: &DataUnit, fee: f32) 
        -> Result<Self, Box<dyn Error>>
    {
        let data_hashes = data.hashes()?;
        let data_length = data.len()?;
        Ok(Page
        {
            id,
            site,

            data_hashes,
            data_length,
            fee,
        })
    }

    pub fn cost(&self) -> f32
    {
        // Bytes used into megabytes
        self.data_length as f32 / PAGE_CHUNK_SIZE as f32
    }

    pub fn is_data_valid(&self, data: &DataUnit) 
        -> Result<(), Box<dyn Error>>
    {
        let hashes = data.hashes()?;
        if hashes.len() != self.data_hashes.len() {
            return Err(ErrorMessage::new("Missmatched data length"));
        }

        for i in 0..hashes.len() 
        {
            if hashes[i] != self.data_hashes[i] {
                return Err(ErrorMessage::new("Incorrect data"));
            }
        }

        Ok(())
    }

}

impl TransactionContent for Page
{

    fn get_fee(&self) -> f32
    {
        self.fee
    }

    fn validate(&self, inputs: &Vec<Input>) 
        -> Result<TransactionValidationResult, Box<dyn Error>>
    {
        if !inputs.iter().any(|x| x.get_address() == self.site) {
            return Ok(TransactionValidationResult::Negative);
        }

        let total_input = inputs.iter().fold(0.0, |acc, x| acc + x.amount);
        if total_input != self.cost() + self.fee {
            return Ok(TransactionValidationResult::Negative);
        }

        let expected_hash_count = self.cost().ceil() as usize;
        if self.data_hashes.len() != expected_hash_count {
            return Ok(TransactionValidationResult::Negative);
        }

        Ok(TransactionValidationResult::Ok)
    }

    fn update_wallet_status(&self, _address: &Hash, mut status: WalletStatus,
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

        if is_block_winner {
            status.balance += self.fee;
        }

        Ok(status)
    }

    fn get_to_addresses(&self) -> Vec<Hash>
    {
        vec![self.site]
    }

    fn get_id(&self) -> u32
    {
        self.id
    }

}

