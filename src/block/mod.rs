mod page;
mod transaction;
mod chain;
pub use page::{Page, PageHeader, DataFormat};
pub use transaction::{Transaction, TransactionHeader};
pub use chain::{BlockChain, Branch, BlockChainAddResult};
use crate::wallet::{Wallet, PublicWallet, WalletStatus};
use crate::error::Error;

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use rsa::BigUint;
use std::time::SystemTime;
use std::collections::HashSet;
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

    fn calculate_target() -> [u8; HASH_LEN]
    {
        // FIXME: Do actually hash target calc
        MIN_TARGET
    }

    pub fn calculate_reward(&self) -> f32
    {
        // FIXME: do real reward calc
        10.0
    }

    pub fn new<W: Wallet>(chain_or_none: Option<&Branch>, raward_to: &W) -> Result<Self, Error>
    {
        let mut prev_block_id = 0;
        let mut prev_block_hash = [0u8; HASH_LEN];
        let mut target = MIN_TARGET;
        if chain_or_none.is_some()
        {
            let chain = chain_or_none.unwrap();
            let top = chain.top();
            target = Self::calculate_target();
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

    pub fn validate_target(&self) -> bool
    {
        self.target == Self::calculate_target()
    }

    pub fn validate(&self) -> bool
    {
        self.validate_pow() 
            && self.validate_target()
    }

    pub fn get_addresses_used(&self) -> Vec<[u8; HASH_LEN]>
    {
        let mut addresses_in_use = HashSet::<[u8; HASH_LEN]>::new();
        addresses_in_use.insert(self.raward_to);
        
        for transaction in &self.transactions
        {
            addresses_in_use.insert(transaction.get_from_address());
            addresses_in_use.insert(transaction.header.to);
        }

        addresses_in_use.into_iter().collect::<Vec<_>>()
    }

    pub fn update_wallet_status(&self, address: &[u8; HASH_LEN], status: &mut WalletStatus)
    {
        if &self.raward_to == address {
            status.balance += self.calculate_reward()
        }

        for transaction in &self.transactions
        {
            let header = &transaction.header;
            if &transaction.get_from_address() == address
            {
                status.balance -= header.amount + header.transaction_fee;
                status.max_id = std::cmp::max(status.max_id, header.id);
            }

            if &header.to == address {
                status.balance += header.amount;
            }

            if &self.raward_to == address {
                status.balance += header.transaction_fee;
            }
        }

        // TODO: Pages
    }

}

#[cfg(test)]
mod tests
{

    /*
    use super::*;
    use crate::logger::{Logger, LoggerLevel};
    use crate::wallet::PrivateWallet;
    use crate::miner;
    use chain::BlockChain;
    use std::path::PathBuf;
    */

    #[test]
    fn test_block()
    {
        /*
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::new(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();

        let block_a = miner::mine_block(Block::new(chain.current_branch(), &wallet).expect("Can create block"));
        chain.add(&block_a, &mut logger);
        assert_eq!(block_a.block_id, 1);
        assert_eq!(block_a.raward_to, wallet.get_address());
        assert_eq!(block_a.prev_hash, [0u8; HASH_LEN]);
        assert_eq!(block_a.validate_pow(), true);
        block_a.validate(chain.current_branch()).expect("Is valid");
        
        let mut block_c = Block::new_debug(3, [0u8; HASH_LEN]);
        block_c.target = [0xFFu8; HASH_LEN];
        block_c = miner::mine_block(block_c);
        assert_eq!(block_c.is_next_block(&block_a).is_ok(), false);
        assert_eq!(block_c.validate(chain.current_branch()).err().expect("Not valid"), Error::PrevNone);
        
        let mut block_b = Block::new(chain.current_branch(), &wallet).expect("Can create block");
        block_b.add_transaction(Transaction::for_chain(chain.current_branch(), &wallet, &other, 2.5, 0.3)
            .expect("Can create transaction"));

        block_b = miner::mine_block(block_b);
        chain.add(&block_b, &mut logger);
        assert_eq!(block_b.block_id, 2);
        assert_eq!(block_b.raward_to, wallet.get_address());
        assert_eq!(block_b.prev_hash, block_a.hash().expect("Hash worked"));
        assert_eq!(block_b.validate(chain.current_branch()).is_ok(), true);
        
        assert_eq!(block_c.validate(chain.current_branch()).err().expect("Not valid"), Error::PrevInvalidHash);
        block_c.prev_hash = block_b.hash().expect("Hash worked");
        block_c.timestamp = block_b.timestamp + 1;
        assert_eq!(block_c.validate(chain.current_branch()).err().expect("Not valid"), Error::InvalidTarget);
        */
    }

}
