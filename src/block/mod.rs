mod page;
mod transaction;
mod chain;
pub use page::{Page, PageHeader, DataFormat};
pub use transaction::{Transaction, TransactionHeader};
pub use chain::BlockChain;
use crate::wallet::{PublicWallet, Wallet};
use crate::error::Error;

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use rsa::BigUint;
use std::collections::HashMap;
use std::time::SystemTime;
use bincode;
use slice_as_array;

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;
type Signature = [u8; PUB_KEY_LEN];
type Hash = [u8; HASH_LEN];

const BLOCK_SIZE: usize = 16 * 1024 * 1024; // 16 MB
const MIN_TARGET: [u8; HASH_LEN] = 
[
    0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
    0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
    0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
    0xFFu8, 0x00u8,
];

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block
{
    pub prev_hash: Hash,
    pub block_id: u64,
    pub raward_to: Hash,

    pub pages: Vec<Page>,
    pub transactions: Vec<Transaction>,
    pub timestamp: u128,
    pub target: Hash,
    pub pow: u64, // TODO: This should be a correct size
}

fn current_timestamp() -> u128
{
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()
}

impl Block
{

    fn _time_for_last_ten_blocks(chain: &BlockChain, top: &Block) -> u128
    {
        let mut current_block = top.clone();
        for _ in 0..10
        {
            let next = chain.block(current_block.block_id - 1);
            if next.is_none() {
                break;
            }

            current_block = next.unwrap().clone();
        }

        return top.timestamp - current_block.timestamp;
    }

    fn calculate_target(_chain: &BlockChain, _top_or_none: Option<&Block>) -> [u8; HASH_LEN]
    {
        // FIXME: Do actually hash target calc
        MIN_TARGET
    }

    pub fn calculate_reward(&self) -> f64
    {
        // FIXME: do real reward calc
        10f64
    }

    pub fn new<W: Wallet>(chain: &BlockChain, raward_to: &W) -> Result<Self, Error>
    {
        let top_or_none = chain.top();
        let mut prev_block_id: u64 = 0;
        let mut prev_block_hash = [0u8; HASH_LEN];
        let target = Self::calculate_target(chain, top_or_none);
        
        if top_or_none.is_some()
        {
            let top = top_or_none.unwrap();
            prev_block_id = top.block_id;
            prev_block_hash = top.hash()?;
        }

        let timestamp = current_timestamp();
        Ok(Block
        {
            prev_hash: prev_block_hash,
            block_id: prev_block_id + 1,
            raward_to: raward_to.get_address(),

            pages: Vec::new(),
            transactions: Vec::new(),
            timestamp: timestamp,
            target: target,
            pow: 0,
        })
    }

    pub fn new_debug(block_id: u64, prev_hash: Hash) -> Self
    {
        Block
        {
            prev_hash,
            block_id,
            raward_to: [0u8; HASH_LEN],

            pages: Vec::new(),
            transactions: Vec::new(),
            timestamp: current_timestamp(),
            target: MIN_TARGET,
            pow: 0,
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

    pub fn as_bytes(&self) -> Result<Vec<u8>, Error>
    {
        let bytes_or_error = bincode::serialize(self);
        if bytes_or_error.is_err() {
            return Err(Error::Other(bytes_or_error.err().unwrap().to_string()));
        }

        let bytes = bytes_or_error.unwrap();
        if bytes.len() > BLOCK_SIZE {
            Err(Error::BlockTooLarge)
        } else {
            Ok(bytes)
        }
    }

    pub fn hash(&self) -> Result<Hash, Error>
    {
        let mut hasher = Sha256::default();
        let bytes = self.as_bytes()?;
        hasher.update(&bytes);

        let hash = hasher.clone().finalize();
        Ok( *slice_as_array!(&hash[0..HASH_LEN], [u8; HASH_LEN]).unwrap() )
    }

    fn validate_transactions(&self, chain: &BlockChain) -> Result<(), Error>
    {
        let mut account_map = HashMap::<[u8; PUB_KEY_LEN], f64>::new();
        for transaction in &self.transactions
        {
            if !account_map.contains_key(&transaction.header.from) {
                account_map.insert(transaction.header.from, 0f64);
            }
            *account_map.get_mut(&transaction.header.from).unwrap() += transaction.header.amount + transaction.header.transaction_fee;

            let wallet = PublicWallet::from_public_key_e(transaction.header.from, transaction.e);
            let header = transaction.header.hash().unwrap();
            if !wallet.varify(&header, &transaction.signature) {
                return Err(Error::InvalidTransactionSignature);
            }
        }

        for page in &self.pages 
        {
            if !account_map.contains_key(&page.header.site_id) {
                account_map.insert(page.header.site_id, 0f64);
            }
            *account_map.get_mut(&page.header.site_id).unwrap() += page.header.page_fee;

            let wallet = PublicWallet::from_public_key_e(page.header.site_id, page.e);
            let header = page.header.hash().unwrap();
            if !wallet.varify(&header, &page.signature) {
                return Err(Error::InvalidPageSignature);
            }
        }

        for (public_key, balance_out) in &account_map
        {
            let wallet = PublicWallet::from_public_key(*public_key);
            let balance = 0f64; //chain.lockup_wallet_status(&wallet).balance;
            if balance < *balance_out {
                return Err(Error::InvalidBalance);
            }
        }

        Ok(())
    }

    pub fn is_next_block(&self, prev: &Block) -> Result<(), Error>
    {
        if self.block_id > 1
        {
            if self.block_id != prev.block_id + 1 {
                return Err(Error::NotNextBlock);
            }

            if self.prev_hash != prev.hash()? {
                return Err(Error::PrevInvalidHash);
            }

            let now = current_timestamp();
            if self.timestamp < prev.timestamp || self.timestamp > now {
                return Err(Error::InvalidTimestamp);
            }
        }

        Ok(())
    }

    pub fn validate(&self, chain: &BlockChain) -> Result<(), Error>
    {
        if self.block_id > 1
        {
            let last_block_or_none = chain.block(self.block_id - 1);
            if last_block_or_none.is_none() {
                return Err(Error::PrevNone);
            }

            let last_block = last_block_or_none.unwrap();
            self.is_next_block(last_block)?;

            let expected_target = Self::calculate_target(chain, Some( last_block ));
            if self.target != expected_target {
                return Err(Error::InvalidTarget);
            }
        }

        self.validate_transactions(chain)
    }

    pub fn validate_pow(&self) -> bool
    {
        let hash_or_none = self.hash();
        if hash_or_none.is_err()
        {
            println!("Faild to hash: {}", hash_or_none.err().unwrap().to_string());
            return false;
        }

        // Validate POW
        let hash = hash_or_none.ok().unwrap();
        let hash_num = BigUint::from_bytes_le(&hash);
        let target_num = BigUint::from_bytes_le(&self.target);
        if hash_num > target_num {
            return false;
        }

        true
    }

}
