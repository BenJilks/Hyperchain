use super::{TransactionHeader, TransactionValidationResult};
use crate::wallet::WalletStatus;
use crate::config::Hash;

use serde::{Serialize, Deserialize};
use std::error::Error;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Page
{
    pub id: u32,
    pub data_hashes: Vec<Hash>,
    pub data_length: u32,
    pub fee: f32,
}

impl Page
{

    pub fn new(id: u32, data_hashes: Vec<Hash>, data_length: u32, fee: f32) -> Self
    {
        Page
        {
            id,
            data_hashes,
            data_length,
            fee,
        }
    }

    pub fn cost(&self) -> f32
    {
        // Bytes used into megabytes
        self.data_length as f32 / (1000.0 * 1000.0)
    }

}

impl TransactionHeader for Page
{

    fn validate(&self) -> Result<TransactionValidationResult, Box<dyn Error>>
    {
        let expected_hash_count = self.cost().ceil() as usize;
        if self.data_hashes.len() != expected_hash_count {
            return Ok(TransactionValidationResult::Negative);
        }

        Ok(TransactionValidationResult::Ok)
    }

    fn update_wallet_status(&self, _address: &Hash, mut status: WalletStatus,
                            is_from_address: bool, is_block_winner: bool)
        -> Option<WalletStatus>
    {
        if is_from_address
        {
            status.balance -= self.cost() + self.fee;
            if self.id <= status.max_id {
                return None;
            }
            status.max_id = self.id;
        }

        if is_block_winner {
            status.balance += self.fee;
        }

        Some(status)
    }

}
