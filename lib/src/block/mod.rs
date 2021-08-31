pub mod validate;
pub mod transactions;
pub mod target;
use crate::transaction::Transaction;
use crate::page::Page;
use crate::chain::BlockChain;
use crate::wallet::Wallet;
use target::{calculate_target, Target};

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use std::error::Error;
use std::fmt::Display;
use bincode;
use slice_as_array;

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;
pub type Signature = [u8; PUB_KEY_LEN];
pub type Hash = [u8; HASH_LEN];

const BLOCK_SIZE: usize = 16 * 1024 * 1024; // 16 MB

pub fn current_timestamp() -> u128
{
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Block
{
    pub prev_hash: Hash,
    pub block_id: u64,
    pub raward_to: Hash,

    pub pages: Vec<Page>,
    pub transactions: Vec<Transaction>,
    pub timestamp: u128,
    pub target: Target,
    pub pow: u64, // TODO: This should be a correct size
}

impl std::fmt::Debug for Block
{

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        f.debug_struct("Block")
            .field("block_id", &self.block_id)
            .field("timestamp", &self.timestamp)
            .field("target", &self.target)
            .field("pow", &self.pow)
            .finish()
    }

}

#[derive(Debug)]
pub enum BlockError
{
    BlockTooLarge,
}

impl Error for BlockError 
{
}

impl Display for BlockError
{

    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
    {
        match self
        {
            BlockError::BlockTooLarge => write!(f, "Block is too large"),
        }
    }

}

impl Block
{

    pub fn calculate_reward(&self) -> f32
    {
        // FIXME: do real reward calc
        10.0
    }

    pub fn new<W: Wallet>(chain: &BlockChain, raward_to: &W) -> Result<Self, Box<dyn Error>>
    {
        let (sample_start, sample_end) = chain.take_sample();
        let target = calculate_target(sample_start, sample_end);
        let (prev_block_id, prev_block_hash) = 
            match chain.top()
            {
                Some(top) => (Some(top.block_id), top.hash()?),
                None => (None, [0u8; HASH_LEN]),
            };

        let block_id =
            match prev_block_id
            {
                Some(id) => id + 1,
                None => 0,
            };

        let timestamp = current_timestamp();
        Ok(Block
        {
            prev_hash: prev_block_hash,
            block_id: block_id,
            raward_to: raward_to.get_address(),

            pages: Vec::new(),
            transactions: Vec::new(),
            timestamp: timestamp,
            target: target,
            pow: 0,
        })
    }

    pub fn add_page(&mut self, page: Page)
    {
        self.pages.push(page);
    }

    pub fn add_transaction(&mut self, transaction: Transaction)
    {
        self.transactions.push(transaction);
    }

    pub fn as_bytes(&self) -> Result<Vec<u8>, Box<dyn Error>>
    {
        let bytes = bincode::serialize(self)?;
        if bytes.len() > BLOCK_SIZE {
            Err(Box::new(BlockError::BlockTooLarge))
        } else {
            Ok(bytes)
        }
    }

    pub fn hash(&self) -> Result<Hash, Box<dyn Error>>
    {
        let mut hasher = Sha256::default();
        let bytes = self.as_bytes()?;
        hasher.update(&bytes);

        let hash = hasher.clone().finalize();
        Ok( *slice_as_array!(&hash[0..HASH_LEN], [u8; HASH_LEN]).unwrap() )
    }

}
