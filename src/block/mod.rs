pub mod validate;
pub mod transactions;
pub mod target;
use crate::transaction::Transaction;
use crate::page::Page;
use crate::chain::branch::Branch;
use crate::wallet::Wallet;
use crate::error::Error;
use target::{calculate_target, Target, BLOCK_SAMPLE_SIZE};

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use bincode;
use slice_as_array;

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;
pub type Signature = [u8; PUB_KEY_LEN];
pub type Hash = [u8; HASH_LEN];

const BLOCK_SIZE: usize = 16 * 1024 * 1024; // 16 MB

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
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

fn current_timestamp() -> u128
{
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()
}

impl Block
{

    pub fn calculate_reward(&self) -> f32
    {
        // FIXME: do real reward calc
        10.0
    }

    pub fn new<W: Wallet>(chain_or_none: Option<&Branch>, raward_to: &W) -> Result<Self, Error>
    {
        let mut prev_block_id = 0;
        let mut prev_block_hash = [0u8; HASH_LEN];
        let mut target = calculate_target(None, None);
        if chain_or_none.is_some()
        {
            let chain = chain_or_none.unwrap();
            let top = chain.top();
            let sample_start = 
                if top.block_id < BLOCK_SAMPLE_SIZE {
                    None
                } else {
                    chain.block(top.block_id - BLOCK_SAMPLE_SIZE)
                };

            target = calculate_target(sample_start, Some(top));
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
            target: calculate_target(None, None),
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

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::validate::BlockValidate;
    use super::transactions::BlockTransactions;
    use crate::logger::{Logger, LoggerLevel};
    use crate::wallet::{PrivateWallet, WalletStatus};
    use crate::miner;
    use std::path::PathBuf;

    #[test]
    fn test_block_verify()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();

        let mut block = Block::new(None, &wallet).expect("Can create block");
        let transaction = Transaction::for_chain(None, &wallet, &other, 4.0, 1.0)
            .expect("Create transaction");
        block.add_transaction(transaction);

        assert_eq!(block.is_pow_valid(), false);
        assert_eq!(block.is_target_valid(None, None), true);
        assert_eq!(block.is_valid(None, None), false);

        block = miner::mine_block(block);
        assert_eq!(block.is_pow_valid(), true);
        assert_eq!(block.is_valid(None, None), true);

        {
            let mut wallet_status = WalletStatus::default();
            block.update_wallet_status(&wallet.get_address(), &mut wallet_status);
            assert_eq!(wallet_status.balance, block.calculate_reward() - 4.0);
            assert_eq!(wallet_status.max_id, 1);
        }

        {
            let mut wallet_status = WalletStatus::default();
            block.update_wallet_status(&other.get_address(), &mut wallet_status);
            assert_eq!(wallet_status.balance, 4.0);
            assert_eq!(wallet_status.max_id, 0);
        }

        let addresses_used = block.get_addresses_used();
        assert_eq!(addresses_used.len(), 2);
        assert_eq!(addresses_used.contains(&wallet.get_address()), true);
        assert_eq!(addresses_used.contains(&other.get_address()), true);
    }

}

