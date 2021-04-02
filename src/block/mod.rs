mod page;
mod transaction;
mod chain;
pub use page::Page;
pub use transaction::Transaction;
pub use chain::BlockChain;

use sha2::{Sha256, Digest};
use rsa::{RSAPublicKey, PublicKey, PaddingScheme, BigUint};
use serde::{Serialize, Deserialize};
use bincode;
use slice_as_array;

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;
type Signature = [u8; PUB_KEY_LEN];
type Hash = [u8; HASH_LEN];

const BLOCK_SIZE: usize = 16 * 1024 * 1024; // 16 MB

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block
{
    pub prev_hash: Hash,
    pub block_id: u64,
    pub raward: u16,

    #[serde(with = "BigArray")]
    pub raward_to: Signature,

    pub pages: Vec<Page>,
    pub transactions: Vec<Transaction>,
    pub difficulty: u16, // TODO: This should be a correct size
    pub pow: u16, // TODO: This should be a correct size
}

impl Block
{

    pub fn new(prev: Option<&Block>, raward_to: Signature) -> Option<Self>
    {
        let prev_block_id = if prev.is_some() { prev.unwrap().block_id } else { 0 };
        let prev_block_hash = if prev.is_some() { prev.unwrap().hash()? } else { [0u8; HASH_LEN] };

        Some(Block
        {
            prev_hash: prev_block_hash,
            block_id: prev_block_id + 1,
            raward: 10, // TODO: This need to be done correctly
            raward_to: raward_to,

            pages: Vec::new(),
            transactions: Vec::new(),
            difficulty: 10,
            pow: 0,
        })
    }

    pub fn from_chain(chain: &BlockChain, raward_to: Signature) -> Option<Self>
    {
        let top = chain.top();
        if top.is_none() {
            Self::new(None, raward_to)
        } else {
            Self::new(Some( top.as_ref().unwrap() ), raward_to)
        }
    }

    pub fn add_page(&mut self, page: Page)
    {
        self.pages.push(page);
    }

    pub fn add_transaction(&mut self, transaction: Transaction)
    {
        self.transactions.push(transaction);
    }

    pub fn as_bytes(&self) -> Option<Vec<u8>>
    {
        let bytes_or_error = bincode::serialize(self);
        if bytes_or_error.is_err() {
            return None;
        }

        let bytes = bytes_or_error.unwrap();
        if bytes.len() > BLOCK_SIZE {
            None
        } else {
            Some( bytes )
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self>
    {
        let result_or_error = bincode::deserialize::<Self>(bytes);
        if result_or_error.is_err() {
            return None;
        }

        return Some( result_or_error.unwrap() );
    }

    pub fn hash(&self) -> Option<Hash>
    {
        let bytes = self.as_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);

        let hash = hasher.finalize();
        return Some( *slice_as_array!(&hash[0..HASH_LEN], [u8; HASH_LEN]).unwrap() );
    }

    /*
    fn print_as_bits(arr: &[u8])
    {
        for byte in arr
        {
            for i in 0..8 {
                print!("{}", (byte >> i) & 0x1);
            }
        }
        println!();
    }
    */

    fn validate_transactions(&self, _chain: &BlockChain) -> bool
    {
        for transaction in &self.transactions
        {
            let from = RSAPublicKey::new(BigUint::from_bytes_le(&transaction.header.from), BigUint::from_bytes_le(&transaction.e)).unwrap();
            let header = transaction.header.hash().unwrap();
            if from.verify(PaddingScheme::new_pkcs1v15_sign(None), &header, &transaction.signature).is_err() {
                return false;
            }
        }

        // TODO: Varify balance
        // chain.lookup(&mut |block: &Block|
        // {
        // });

        return true;
    }

    pub fn validate(&self, chain: &BlockChain) -> bool
    {
        if self.block_id > 1
        {
            let last_block_or_none = chain.block(self.block_id - 1);
            if last_block_or_none.is_none() 
            {
                println!("prev is none");
                return false;
            }

            let last_block = last_block_or_none.unwrap();
            if self.block_id != last_block.block_id + 1 
            {
                println!("prev is not the last block");
                return false;
            }

            let prev_hash = last_block.hash();
            if prev_hash.is_none() 
            {
                println!("prev faild to hash");
                return false;
            }

            if self.prev_hash != prev_hash.unwrap() 
            {
                println!("prev hash does not match this hash");
                return false;
            }
        }

        let hash_or_none = self.hash();
        if hash_or_none.is_none() 
        {
            println!("faild to hash");
            return false;
        }

        // Validate POW
        let hash = hash_or_none.unwrap();
        for i in 0..(self.difficulty / 8 + 1) as usize
        {
            if i < (self.difficulty as usize / 8) && hash[i] != 0 {
                return false;
            }

            let zero_count = self.difficulty % 8;
            let mask = (1u8 << zero_count) - 1;
            if hash[i] & mask != 0 {
                return false;
            }
        }

        return self.validate_transactions(chain);
    }

}
