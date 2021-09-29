pub mod transfer;
pub mod page;
pub mod builder;
use transfer::Transfer;
use page::Page;
use crate::wallet::WalletStatus;
use crate::wallet::public_wallet::{PublicWallet, WalletValidationResult};
use crate::config::{Signature, Hash, HASH_LEN};

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::error::Error;

big_array! { BigArray; }

#[derive(Debug, PartialEq)]
pub enum TransactionValidationResult
{
    Ok,
    Negative,
    Wallet(WalletValidationResult),
}

pub trait TransactionContent
{

    fn get_fee(&self) -> f32;

    fn validate(&self, inputs: &Vec<Input>)
        -> Result<TransactionValidationResult, Box<dyn Error>>;

    fn update_wallet_status(&self, address: &Hash, status: WalletStatus, 
                            from_amount: f32, is_block_winner: bool)
        -> Option<WalletStatus>;

    fn get_to_addresses(&self) -> Vec<Hash>;

    fn get_id(&self) -> u32;

}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Input
{
    #[serde(with = "BigArray")]
    pub from: Signature,

    pub e: [u8; 3],
    pub amount: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TransactionHeader<C>
    where C: TransactionContent
{
    pub content: C,
    pub inputs: Vec<Input>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Transaction<C>
    where C: TransactionContent
{
    pub header: TransactionHeader<C>,
    pub signatures: HashMap<Hash, Vec<u8>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TransactionVariant
{
    Transfer(Transaction<Transfer>),
    Page(Transaction<Page>),
}

impl std::fmt::Display for TransactionValidationResult
{

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self
        {
            TransactionValidationResult::Ok => write!(f, "Ok"),
            TransactionValidationResult::Negative => write!(f, "Can't have negitive transfer amounts"),
            TransactionValidationResult::Wallet(wallet) => write!(f, "{}", wallet),
        }
    }

}

impl<C> TransactionHeader<C>
    where C: TransactionContent + Serialize
{

    pub fn hash(&self) -> Result<Vec<u8>, Box<dyn Error>>
    {
        let result = bincode::serialize(self)?;
        let mut hasher = Sha256::new();
        hasher.update(&result);
        Ok( hasher.finalize().to_vec() )
    }

}

impl Input
{

    pub fn get_address(&self) -> Hash
    {
        let mut hasher = Sha256::new();
        hasher.update(&self.from);

        let hash = hasher.finalize().to_vec();
        *slice_as_array!(&hash, [u8; HASH_LEN]).unwrap()
    }

}

impl<C> Transaction<C>
    where C: TransactionContent + Serialize
{

    pub fn new(header: TransactionHeader<C>, signatures: HashMap<Hash, Vec<u8>>) -> Self
    {
        Self
        {
            header,
            signatures,
        }
    }

    pub fn hash(&self) -> Result<Vec<u8>, Box<dyn Error>>
    {
        self.header.hash()
    }

    pub fn fee_per_byte(&self) -> Result<f32, Box<dyn Error>>
    {
        let total_fee = self.header.content.get_fee();
        let size_in_bytes = bincode::serialize(&self.header)?.len();
        Ok(total_fee / size_in_bytes as f32)
    }

    pub fn update_wallet_status(&self, address: &Hash, status: WalletStatus, 
                                is_block_winner: bool)
        -> Option<WalletStatus>
    {
        let from = self.header.inputs.iter().find(|x| &x.get_address() == address);
        let from_amount = match from
        {
            Some(input) => input.amount,
            None => 0.0,
        };

        self.header.content.update_wallet_status(address, status,
            from_amount, is_block_winner)
    }

    pub fn validate_content(&self) -> Result<TransactionValidationResult, Box<dyn Error>>
    {
        let header_result = self.header.content.validate(&self.header.inputs)?;
        if header_result != TransactionValidationResult::Ok {
            return Ok(header_result);
        }

        for input in &self.header.inputs
        {
            let signature = &self.signatures[&input.get_address()];
            let wallet = PublicWallet::from_public_key_e(input.from, input.e);
            let wallet_result = wallet.verify(&self.hash()?, &signature)?;

            if wallet_result != WalletValidationResult::Ok {
                return Ok(TransactionValidationResult::Wallet(wallet_result));
            }
        }

        Ok(TransactionValidationResult::Ok)
    }

    pub fn get_from_addresses(&self) -> Vec<Hash>
    {
        let mut addresses = Vec::new();
        for input in &self.header.inputs {
            addresses.push(input.get_address());
        }

        addresses
    }

    pub fn get_addresses_used(&self) -> Vec<Hash>
    {
        let mut inputs = self.get_from_addresses();
        let mut outputs = self.header.content.get_to_addresses();
        inputs.append(&mut outputs);
        inputs
    }

    pub fn get_id(&self) -> u32
    {
        self.header.content.get_id()
    }

}

