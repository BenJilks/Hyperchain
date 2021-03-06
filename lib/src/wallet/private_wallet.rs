/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use super::Wallet;
use super::public_wallet::PublicWallet;
use crate::hash::Signature;
use crate::config::PUB_KEY_LEN;
use rsa::{RSAPrivateKey, PaddingScheme, PrivateKeyEncoding, PublicKeyParts};
use rand::rngs::OsRng;
use std::fs::File;
use std::path::PathBuf;
use std::io::{Read, Write};
use std::error::Error;
use slice_as_array;

pub struct PrivateWallet
{
    key: RSAPrivateKey,
}

impl Wallet for PrivateWallet
{

    fn get_public_key(&self) -> Signature
    {
        let bytes = self.key.n().to_bytes_le();
        Signature::from(&bytes)
    }

}

impl PrivateWallet
{

    pub fn new() -> rsa::errors::Result<Self>
    {
        let mut rng = OsRng;
        let key = RSAPrivateKey::new(&mut rng, PUB_KEY_LEN * 8)?;

        Ok(Self {
            key,
        })
    }

    pub fn open_temp(id: u32) 
        -> Result<Self, Box<dyn Error>>
    {
        let file_path = std::env::temp_dir().join(format!("{}.wallet", id));
        if file_path.as_path().exists() {
            return Ok(Self::read_from_file(&file_path)?);
        }

        let wallet = Self::new()?;
        wallet.write_to_file(&file_path)?;
        Ok(wallet)
    }

    pub fn serialize(&self) -> Vec<u8>
    {
        self.key.to_pkcs8().unwrap()
    }

    pub fn deserialize(buffer: Vec<u8>) -> Result<Self, Box<dyn Error>>
    {
        let key = RSAPrivateKey::from_pkcs8(&buffer)?;
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

    pub fn read_from_file(path: &PathBuf) -> std::io::Result<Self>
    {
        let mut file = File::open(path)?;
        let mut buffer = Vec::<u8>::new();
        file.read_to_end(&mut buffer)?;
        info!("Opened wallet '{:?}'", path);

        let key = RSAPrivateKey::from_pkcs8(&buffer).unwrap();
        Ok(Self
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

