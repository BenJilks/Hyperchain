use crate::wallet::Wallet;
use crate::wallet::private_wallet::PrivateWallet;
use crate::wallet::public_wallet::{PublicWallet, WalletValidationResult};
use crate::config::{Signature, Hash, PUB_KEY_LEN, HASH_LEN};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use bincode;

use std::string::ToString;
use std::error::Error;

big_array! { BigArray; }

#[derive(Debug, PartialEq)]
pub enum TransferValidationResult
{
    Ok,
    Negative,
    Wallet(WalletValidationResult),
}

impl std::fmt::Display for TransferValidationResult
{

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self
        {
            TransferValidationResult::Ok => write!(f, "Ok"),
            TransferValidationResult::Negative => write!(f, "Can't have negitive transfer amounts"),
            TransferValidationResult::Wallet(wallet) => write!(f, "{}", wallet),
        }
    }

}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TransferHeader
{
    pub id: u32,
    
    #[serde(with = "BigArray")]
    pub from: Signature,
    
    pub to: Hash,
    pub amount: f32,
    pub fee: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Transfer
{
    pub header: TransferHeader,

    #[serde(with = "BigArray")]
    pub signature: Signature,
    
    pub e: [u8; 3],
}

impl TransferHeader
{

    pub fn hash(&self) -> Result<Vec<u8>, Box<dyn Error>>
    {
        let result = bincode::serialize(self)?;
        let mut hasher = Sha256::new();
        hasher.update(&result);
        Ok( hasher.finalize().to_vec() )
    }

}

impl Transfer
{

    pub fn new(id: u32, from: &PrivateWallet, to: Hash, amount: f32, fee: f32) -> Self
    {
        let header = TransferHeader 
        { 
            id,
            from: from.get_public_key(),
            to,
            amount,
            fee,
        };

        let signature_vec = from.sign(&header.hash().unwrap()).unwrap();
        let signature = *slice_as_array!(&signature_vec, [u8; PUB_KEY_LEN]).unwrap();
        Self
        {
            header,
            signature, 
            e: from.get_e(),
        }
    }

    pub fn validate_content(&self) -> Result<TransferValidationResult, Box<dyn Error>>
    {
        if self.header.amount < 0.0 {
            return Ok(TransferValidationResult::Negative);
        }

        if self.header.fee < 0.0 {
            return Ok(TransferValidationResult::Negative);
        }

        let wallet = PublicWallet::from_public_key_e(self.header.from, self.e);
        let header = self.header.hash()?;
        match wallet.verify(&header, &self.signature)?
        {
            WalletValidationResult::Ok => Ok(TransferValidationResult::Ok),
            result => Ok(TransferValidationResult::Wallet(result)),
        }
    }

    pub fn get_from_address(&self) -> [u8; HASH_LEN]
    {
        let mut hasher = Sha256::new();
        hasher.update(&self.header.from);

        let hash = hasher.finalize().to_vec();
        *slice_as_array!(&hash, [u8; HASH_LEN]).unwrap()
    }

}

impl ToString for Transfer
{

    fn to_string(&self) -> String
    {
        format!("{}... --[ {} + {}tx ]--> {}...", 
            &base_62::encode(&self.header.from)[0..10],
            self.header.amount,
            self.header.fee,
            &base_62::encode(&self.header.to)[0..10])
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::block::Block;
    use crate::chain::BlockChain;
    use crate::logger::{Logger, LoggerLevel};
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
            let transfer = Transfer::new(0, &wallet, other.get_address(), 2.4, 0.2);
            transfer.header.hash().expect("Hash header");
            assert_eq!(transfer.validate_content().unwrap(), TransferValidationResult::Ok);
            assert_eq!(transfer.to_string(), "aLOExVDb0w... --[ 2.4 + 0.2tx ]--> zCPOqvKFuo...");
        }

        {
            let transfer = Transfer::new(1, &wallet, other.get_address(), -1.6, 0.0);
            assert_ne!(transfer.validate_content().unwrap(), TransferValidationResult::Ok);
        }

        {
            let transfer = Transfer::new(2, &wallet, other.get_address(), 0.0, -0.0001);
            assert_ne!(transfer.validate_content().unwrap(), TransferValidationResult::Ok);
        }
    }

}
