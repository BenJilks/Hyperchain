use super::Wallet;
use crate::config::PUB_KEY_LEN;

use serde::{Serialize, Deserialize};
use rsa::{RSAPublicKey, PublicKey, PaddingScheme, BigUint};
use std::error::Error;

big_array! { BigArray; }

#[derive(Debug, PartialEq)]
pub enum WalletValidationResult
{
    Ok,
    Signature,
}

impl std::fmt::Display for WalletValidationResult
{

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self
        {
            WalletValidationResult::Ok => write!(f, "Ok"),
            WalletValidationResult::Signature => write!(f, "Signature not valid"),
        }
    }

}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PublicWallet
{
    #[serde(with = "BigArray")]
    public_key: [u8; PUB_KEY_LEN],

    e: Option<[u8; 3]>,
}

impl Wallet for PublicWallet
{

    fn get_public_key(&self) -> [u8; PUB_KEY_LEN]
    {
        self.public_key
    }

}

impl PublicWallet
{

    pub fn from_public_key(public_key: &[u8]) -> Self
    {
        let key = slice_as_array!(public_key, [u8; PUB_KEY_LEN]).expect("Bad hash length");
        Self
        {
            public_key: *key,
            e: None,
        }
    }

    pub fn from_public_key_e(public_key: [u8; PUB_KEY_LEN], e: [u8; 3]) -> Self
    {
        Self
        {
            public_key,
            e: Some( e ),
        }
    }

    pub fn verify(&self, hash: &[u8], signature: &[u8]) -> Result<WalletValidationResult, Box<dyn Error>>
    {
        assert_eq!(self.e.is_none(), false);

        let n = BigUint::from_bytes_le(&self.public_key);
        let e = BigUint::from_bytes_le(&self.e.unwrap());
        let key = RSAPublicKey::new(n, e)?;
        if key.verify(PaddingScheme::new_pkcs1v15_sign(None), hash, signature).is_ok() {
            Ok(WalletValidationResult::Ok)
        } else {
            Ok(WalletValidationResult::Signature)
        }
    }

}

