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

fn current_timestamp() -> u128
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

#[cfg(test)]
mod tests
{

    use super::*;
    use super::validate::{BlockValidate, BlockValidationResult};
    use super::transactions::BlockTransactions;
    use crate::logger::{Logger, LoggerLevel};
    use crate::wallet::WalletStatus;
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::miner;
    use std::path::PathBuf;

    #[test]
    fn test_block_verify()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();
        let chain = BlockChain::new(&mut logger);

        let mut block = Block::new(&chain, &wallet).expect("Can create block");
        let transaction = Transaction::for_chain(&chain, &wallet, &other, 4.0, 1.0)
            .expect("Create transaction");
        block.add_transaction(transaction);

        assert_ne!(block.validate_pow().unwrap(), BlockValidationResult::Ok);
        assert_eq!(block.validate_target(None, None), BlockValidationResult::Ok);
        assert_ne!(block.validate(None, None).unwrap(), BlockValidationResult::Ok);

        block = miner::mine_block(block);
        assert_eq!(block.validate_pow().unwrap(), BlockValidationResult::Ok);
        assert_eq!(block.validate(None, None).unwrap(), BlockValidationResult::Ok);

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

