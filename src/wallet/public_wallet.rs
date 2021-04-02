use crate::block::PUB_KEY_LEN;
use super::Wallet;

use rsa::{RSAPublicKey, PublicKey, PaddingScheme, BigUint};

pub struct PublicWallet
{
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

    pub fn from_public_key(public_key: [u8; PUB_KEY_LEN]) -> Self
    {
        Self
        {
            public_key,
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

    pub fn varify(&self, hash: &[u8], signature: &[u8]) -> bool
    {
        assert_eq!(self.e.is_none(), false);

        let n = BigUint::from_bytes_le(&self.public_key);
        let e = BigUint::from_bytes_le(&self.e.unwrap());
        let key = RSAPublicKey::new(n, e).unwrap();
        key.verify(PaddingScheme::new_pkcs1v15_sign(None), hash, signature).is_err()
    }

}
