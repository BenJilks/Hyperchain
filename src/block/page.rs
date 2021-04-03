use super::{Signature, BlockChain, PUB_KEY_LEN};
use crate::wallet::{Wallet, PrivateWallet};

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use std::io::Read;

big_array! { BigArray; }

pub enum DataFormat
{
    NewRaw = 0,
    DiffRaw = 1,
}

impl DataFormat
{

    pub fn from_u8(id: u8) -> Option<Self>
    {
        match id
        {
            0 => Some( Self::NewRaw ),
            1 => Some( Self::DiffRaw ),
            _ => None,
        }
    }

}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct PageHeader
{
    pub data_format: u8,
    pub page_data: Vec<u8>,

    #[serde(with = "BigArray")]
    pub site_id: Signature,

    pub page_name: String,
    pub page_fee: u32,
}

impl PageHeader
{

    pub fn hash(&self) -> Option<Vec<u8>>
    {
        let result = bincode::serialize(self);
        if result.is_err() {
            return None;
        }

        let mut hasher = Sha256::new();
        hasher.update(&result.unwrap());
        Some( hasher.finalize().to_vec() )
    }

}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Page
{
    pub header: PageHeader,

    #[serde(with = "BigArray")]
    pub signature: Signature,

    pub e: [u8; 3],
}

impl Page
{

    pub fn new(format: DataFormat, data: Vec<u8>, owner: &PrivateWallet, page_name: &str, fee: u32) -> Self
    {
        let header = PageHeader
        {
            data_format: format as u8,
            page_data: data,
            site_id: owner.get_public_key(),
            page_name: page_name.to_owned(),
            page_fee: fee,
        };

        let signature = owner.sign(&header.hash().unwrap()).unwrap();
        Self
        {
            header: header,
            signature: *slice_as_array!(&signature, [u8; PUB_KEY_LEN]).unwrap(),
            e: owner.get_e(),
        }
    }

    pub fn from_file(chain: &BlockChain, new_page: &[u8], owner: &PrivateWallet, page_name: &str, fee: u32) -> Self
    {
        let existing_page_file = chain.page(owner, page_name);
        if existing_page_file.is_none() {
            return Self::new(DataFormat::NewRaw, new_page.to_owned(), owner, page_name, fee);
        }

        let mut existing_page = Vec::<u8>::new();
        let mut diff = Vec::<u8>::new();
        existing_page_file.unwrap().read_to_end(&mut existing_page).unwrap();
        bidiff::simple_diff(&existing_page, new_page, &mut diff).unwrap();
        Self::new(DataFormat::DiffRaw, diff, owner, page_name, fee)
    }

}
