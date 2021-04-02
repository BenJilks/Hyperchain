mod page;
mod transaction;
mod chain;
pub use page::Page;
pub use transaction::Transaction;
pub use chain::BlockChain;
use crate::wallet::{PublicWallet, Wallet};

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use rsa::BigUint;
use std::collections::HashMap;
use std::time::SystemTime;
use num_traits::pow::Pow;
use bincode;
use slice_as_array;

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;
pub const MS_TO_FIND_BLOCK: usize = 1000;
type Signature = [u8; PUB_KEY_LEN];
type Hash = [u8; HASH_LEN];

const BLOCK_SIZE: usize = 16 * 1024 * 1024; // 16 MB
const MIN_TARGET: [u8; HASH_LEN] = 
[
    0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
    0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
    0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8, 0xFFu8,
    0xFFu8, 0xF2u8,
];

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Block
{
    pub prev_hash: Hash,
    pub block_id: u64,

    #[serde(with = "BigArray")]
    pub raward_to: Signature,

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

    fn time_for_last_ten_blocks(chain: &BlockChain, top: &Block) -> u128
    {
        let mut current_block = top.clone();
        for _ in 0..10
        {
            let next = chain.block(current_block.block_id - 1);
            if next.is_none() {
                break;
            }

            current_block = next.unwrap();
        }

        return top.timestamp - current_block.timestamp;
    }

    fn calculate_target(chain: &BlockChain, top_or_none: &Option<Block>) -> [u8; HASH_LEN]
    {
        if top_or_none.is_none() {
            return MIN_TARGET;
        }

        let top = top_or_none.as_ref().unwrap();
        if top.block_id % 10 != 0 {
            return top.target;
        }

        let average_time = std::cmp::max(Self::time_for_last_ten_blocks(chain, top), 10) / 10;

        let target_num = BigUint::from_bytes_le(&top.target);
        let c_2_pow_256 = BigUint::from(2u32).pow(256u32);
        let last_difficualty = c_2_pow_256.clone() / target_num;
        let hash_rate = std::cmp::max(last_difficualty.clone() / average_time, BigUint::from(1u32));

        let new_difficaulty = hash_rate.clone() * MS_TO_FIND_BLOCK;
        let new_target_num = c_2_pow_256.clone() / new_difficaulty;
        let mut new_target = new_target_num.to_bytes_le();
        new_target.resize(HASH_LEN, 0);
        
        println!("{} in {}, {} H/ms", last_difficualty, average_time, hash_rate);
        return *slice_as_array!(&new_target, [u8; HASH_LEN]).unwrap();
    }

    pub fn new(chain: &BlockChain, raward_to: Signature) -> Option<Self>
    {
        let top_or_none = chain.top();
        let mut prev_block_id: u64 = 0;
        let mut prev_block_hash = [0u8; HASH_LEN];
        let target = Self::calculate_target(chain, &top_or_none);

        if top_or_none.is_some()
        {
            let top = top_or_none.unwrap();
            prev_block_id = top.block_id;
            prev_block_hash = top.hash()?;
        }

        let timestamp = current_timestamp();
        Some(Block
        {
            prev_hash: prev_block_hash,
            block_id: prev_block_id + 1,
            raward_to: raward_to,

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

    pub fn calculate_reward(&self) -> u32
    {
        10u32 // FIXME: do real reward calc
    }

    fn validate_transactions(&self, chain: &BlockChain) -> bool
    {
        let mut account_map = HashMap::<[u8; PUB_KEY_LEN], u32>::new();
        for transaction in &self.transactions
        {
            if !account_map.contains_key(&transaction.header.from) {
                account_map.insert(transaction.header.from, 0);
            }
            *account_map.get_mut(&transaction.header.from).unwrap() += transaction.header.amount + transaction.header.transaction_fee;

            let wallet = PublicWallet::from_public_key_e(transaction.header.from, transaction.e);
            let header = transaction.header.hash().unwrap();
            if wallet.varify(&header, &transaction.signature) {
                return false;
            }
        }

        for (public_key, balance_out) in &account_map
        {
            let wallet = PublicWallet::from_public_key(*public_key);
            let balance = wallet.calculate_balance(chain);
            if balance < *balance_out {
                return false;
            }
        }

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

            let now = current_timestamp();
            if self.timestamp < last_block.timestamp || self.timestamp > now {
                return false;
            }

            let expected_target = Self::calculate_target(chain, &Some( last_block ));
            if self.target != expected_target {
                return false;
            }
        }

        return self.validate_transactions(chain);
    }

    pub fn validate_pow(&self) -> bool
    {
        let hash_or_none = self.hash();
        if hash_or_none.is_none() 
        {
            println!("faild to hash");
            return false;
        }

        // Validate POW
        let hash = hash_or_none.unwrap();
        let hash_num = BigUint::from_bytes_le(&hash);
        let target_num = BigUint::from_bytes_le(&self.target);
        if hash_num > target_num {
            return false;
        }

        true
    }

}
