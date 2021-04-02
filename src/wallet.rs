extern crate rsa;
extern crate rand;
use crate::block::{BlockChain, Block, PUB_KEY_LEN};
use rsa::{RSAPrivateKey, PaddingScheme, PrivateKeyEncoding, PublicKeyParts};
use rand::rngs::OsRng;
use std::fs::File;
use std::path::PathBuf;
use std::io::{Read, Write};
use slice_as_array;

pub struct Wallet
{
    key: RSAPrivateKey,
}

impl Wallet
{

    pub fn new() -> rsa::errors::Result<Self>
    {
        let mut rng = OsRng;
        let key = RSAPrivateKey::new(&mut rng, PUB_KEY_LEN * 8)?;

        Ok(Self
        {
            key,
        })
    }

    pub fn write_to_file(&self, path: &PathBuf) -> std::io::Result<()>
    {
        let mut file = File::create(path)?;
        file.write(&self.key.to_pkcs8().unwrap())?;
        Ok(())
    }

    pub fn read_from_file(path: &PathBuf) -> std::io::Result<Self>
    {
        let mut file = File::open(path)?;
        let mut buffer = Vec::<u8>::new();
        file.read_to_end(&mut buffer)?;

        let key = RSAPrivateKey::from_pkcs8(&buffer).unwrap();
        return Ok(Self
        {
            key,
        })
    }

    pub fn sign(&self, digest: &[u8]) -> rsa::errors::Result<Vec<u8>>
    {
        self.key.sign(PaddingScheme::new_pkcs1v15_sign(None), digest)
    }

    pub fn get_public_key(&self) -> [u8; PUB_KEY_LEN]
    {
        let bytes = self.key.n().to_bytes_le();
        *slice_as_array!(&bytes, [u8; PUB_KEY_LEN]).unwrap()
    }

    pub fn get_e(&self) -> [u8; 3]
    {
        let bytes = self.key.e().to_bytes_le();
        *slice_as_array!(&bytes, [u8; 3]).unwrap()
    }

    pub fn calculate_balance(&self, chain: &BlockChain) -> u32
    {
        let mut balance: u32 = 0;
        let pub_key = self.get_public_key();

        chain.lookup(&mut |block: &Block|
        {
            let mut is_miner = false;
            if block.raward_to == pub_key 
            {
                balance += block.raward as u32;
                is_miner = true;
            }

            for transaction in &block.transactions
            {
                if transaction.header.to == pub_key {
                    balance += transaction.header.amount;
                }

                if transaction.header.from == pub_key 
                {
                    balance -= transaction.header.amount;
                    balance -= transaction.header.transaction_fee;
                }

                if is_miner {
                    balance += transaction.header.transaction_fee;
                }
            }
        });

        balance
    }

}
