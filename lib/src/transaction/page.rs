use crate::wallet::private_wallet::PrivateWallet;
use crate::wallet::Wallet;
use crate::config::{Signature, Hash, PUB_KEY_LEN};

use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct PageHeader
{
    pub data_hash: Hash,
    pub data_length: u32,

    #[serde(with = "BigArray")]
    pub site_id: Signature,

    pub fee: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Page
{
    pub header: PageHeader,

    #[serde(with = "BigArray")]
    pub signature: Signature,

    pub e: [u8; 3],
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

impl Page
{

    pub fn new(hash: Hash, length: u32, owner: &PrivateWallet, fee: f32) -> Self
    {
        let header = PageHeader
        {
            data_hash: hash,
            data_length: length,
            site_id: owner.get_public_key(),
            fee: fee,
        };

        let signature = owner.sign(&header.hash().unwrap()).unwrap();
        Self
        {
            header: header,
            signature: *slice_as_array!(&signature, [u8; PUB_KEY_LEN]).unwrap(),
            e: owner.get_e(),
        }
    }

    /*
    pub fn from_file(chain: &BlockChain, new_page: &[u8], owner: &PrivateWallet, page_name: &str, fee: f32) -> Self
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
    */

}
