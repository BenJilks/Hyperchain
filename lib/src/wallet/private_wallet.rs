use super::Wallet;
use super::public_wallet::PublicWallet;
use crate::block::PUB_KEY_LEN;
use crate::logger::{Logger, LoggerLevel};

use rsa::{RSAPrivateKey, PaddingScheme, PrivateKeyEncoding, PublicKeyParts};
use rand::rngs::OsRng;
use std::fs::File;
use std::path::PathBuf;
use std::io::{Read, Write};
use slice_as_array;

pub struct PrivateWallet
{
    key: RSAPrivateKey,
}

impl Wallet for PrivateWallet
{

    fn get_public_key(&self) -> [u8; PUB_KEY_LEN]
    {
        let bytes = self.key.n().to_bytes_le();
        *slice_as_array!(&bytes, [u8; PUB_KEY_LEN]).unwrap()
    }

}

impl PrivateWallet
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

    pub fn as_public(&self) -> PublicWallet
    {
        PublicWallet::from_public_key_e(self.get_public_key(), self.get_e())
    }

    pub fn write_to_file(&self, path: &PathBuf) -> std::io::Result<()>
    {
        let mut file = File::create(path)?;
        file.write(&self.key.to_pkcs8().unwrap())?;
        Ok(())
    }

    pub fn read_from_file<W: Write>(path: &PathBuf, logger: &mut Logger<W>) -> std::io::Result<Self>
    {
        let mut file = File::open(path)?;
        let mut buffer = Vec::<u8>::new();
        file.read_to_end(&mut buffer)?;
        logger.log(LoggerLevel::Info, &format!("Opened wallet '{:?}'", path));

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

    pub fn get_e(&self) -> [u8; 3]
    {
        let bytes = self.key.e().to_bytes_le();
        *slice_as_array!(&bytes, [u8; 3]).unwrap()
    }

}
