pub mod transfer;
pub mod page;
use transfer::Transfer;
use page::Page;
use crate::wallet::{Wallet, WalletStatus};
use crate::wallet::private_wallet::PrivateWallet;
use crate::wallet::public_wallet::{PublicWallet, WalletValidationResult};
use crate::config::{Signature, Hash, PUB_KEY_LEN, HASH_LEN};

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::error::Error;

big_array! { BigArray; }

#[derive(Debug, PartialEq)]
pub enum TransactionValidationResult
{
    Ok,
    Negative,
    Wallet(WalletValidationResult),
}

pub trait TransactionHeader
{

    fn validate(&self) 
        -> Result<TransactionValidationResult, Box<dyn Error>>;

    fn update_wallet_status(&self, address: &Hash, status: WalletStatus, 
                            is_from_address: bool, is_block_winner: bool)
        -> Option<WalletStatus>;

}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Transaction<H>
    where H: TransactionHeader
{
    pub header: H,

    #[serde(with = "BigArray")]
    pub from: Signature,

    #[serde(with = "BigArray")]
    pub signature: Signature,
    
    pub e: [u8; 3],
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

fn hash_header<H>(header: &H) -> Result<Vec<u8>, Box<dyn Error>>
    where H: TransactionHeader + Serialize
{
    let result = bincode::serialize(header)?;
    let mut hasher = Sha256::new();
    hasher.update(&result);
    Ok( hasher.finalize().to_vec() )
}

impl<H> Transaction<H>
    where H: TransactionHeader + Serialize
{

    pub fn new(from: &PrivateWallet, header: H) -> Self
    {
        let signature_vec = from.sign(&hash_header(&header).unwrap()).unwrap();
        let signature = *slice_as_array!(&signature_vec, [u8; PUB_KEY_LEN]).unwrap();
        Self
        {
            header,
            from: from.get_public_key(),
            signature,
            e: from.get_e(),
        }
    }

    pub fn hash(&self) -> Result<Vec<u8>, Box<dyn Error>>
    {
        hash_header(&self.header)
    }

    pub fn update_wallet_status(&self, address: &Hash, status: WalletStatus, 
                                is_block_winner: bool)
        -> Option<WalletStatus>
    {
        let is_from_address = &self.get_from_address() == address;

        self.header.update_wallet_status(address, status,
            is_from_address, is_block_winner)
    }

    pub fn validate_content(&self) -> Result<TransactionValidationResult, Box<dyn Error>>
    {
        let header_result = self.header.validate()?;
        if header_result != TransactionValidationResult::Ok {
            return Ok(header_result);
        }

        let wallet = PublicWallet::from_public_key_e(self.from, self.e);
        let wallet_result = wallet.verify(&self.hash()?, &self.signature)?;
        if wallet_result != WalletValidationResult::Ok {
            return Ok(TransactionValidationResult::Wallet(wallet_result));
        }

        Ok(TransactionValidationResult::Ok)
    }

    pub fn get_from_address(&self) -> [u8; HASH_LEN]
    {
        let mut hasher = Sha256::new();
        hasher.update(&self.from);

        let hash = hasher.finalize().to_vec();
        *slice_as_array!(&hash, [u8; HASH_LEN]).unwrap()
    }

}
