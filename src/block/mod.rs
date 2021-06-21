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

    pub fn calculate_reward(&self) -> f32
    {
        // FIXME: do real reward calc
        10.0
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
        let mut account_map = HashMap::<[u8; PUB_KEY_LEN], f32>::new();
        for transaction in &self.transactions
        {
            if !account_map.contains_key(&transaction.header.from) {
                account_map.insert(transaction.header.from, 0.0);
            }
            *account_map.get_mut(&transaction.header.from).unwrap() += transaction.header.amount + transaction.header.transaction_fee;
            transaction.varify()?
        }

        for page in &self.pages
        {
            if !account_map.contains_key(&page.header.site_id) {
                account_map.insert(page.header.site_id, 0.0);
            }
            *account_map.get_mut(&page.header.site_id).unwrap() += page.header.page_fee;
            // TODO: Varify page
        }

        for (public_key, balance_out) in &account_map
        {
            // TODO: Actually calculate balance
            let wallet = PublicWallet::from_public_key(*public_key);
            let balance = wallet.get_status(chain).balance;
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

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::{Logger, LoggerLevel};
    use crate::PrivateWallet;
    use crate::miner;
    use chain::BlockChain;
    use std::path::PathBuf;

    #[test]
    fn test_block()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::new(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();

        let block_a = miner::mine_block(Block::new(&chain, &wallet).expect("Can create block"));
        chain.add(&block_a, &mut logger);
        assert_eq!(block_a.block_id, 1);
        assert_eq!(block_a.raward_to, wallet.get_address());
        assert_eq!(block_a.prev_hash, [0u8; HASH_LEN]);
        assert_eq!(block_a.validate_pow(), true);
        block_a.validate(&chain).expect("Is valid");
        
        let mut block_c = Block::new_debug(3, [0u8; HASH_LEN]);
        block_c.target = [0u8; HASH_LEN];
        assert_eq!(block_c.is_next_block(&block_a).is_ok(), false);
        assert_eq!(block_c.validate(&chain).err().expect("Not valid"), Error::PrevNone);
        
        let mut block_b = Block::new(&chain, &wallet).expect("Can create block");
        block_b.add_transaction(Transaction::for_block(&chain, &wallet, &other, 2.5, 0.3).expect("Can create transaction"));
        chain.add(&block_b, &mut logger);
        assert_eq!(block_b.block_id, 2);
        assert_eq!(block_b.raward_to, wallet.get_address());
        assert_eq!(block_b.prev_hash, block_a.hash().expect("Hash worked"));
        assert_eq!(block_b.validate_pow(), false);
        block_b.validate(&chain).expect("Is valid");
        
        assert_eq!(block_c.validate(&chain).err().expect("Not valid"), Error::PrevInvalidHash);
        block_c.prev_hash = block_b.hash().expect("Hash worked");
        assert_eq!(block_c.validate(&chain).err().expect("Not valid"), Error::InvalidTimestamp);
        block_c.timestamp = block_b.timestamp + 1;
        assert_eq!(block_c.validate(&chain).err().expect("Not valid"), Error::InvalidTarget);
    }

}
