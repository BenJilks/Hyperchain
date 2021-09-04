pub mod private_wallet;
pub mod public_wallet;
use crate::config::{Hash, HASH_LEN, PUB_KEY_LEN};
use crate::chain::BlockChain;

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WalletStatus
{
    pub balance: f32,
    pub max_id: u32,
}

impl Default for WalletStatus
{

    fn default() -> WalletStatus
    {
        WalletStatus
        {
            balance: 0.0,
            max_id: 0,
        }
    }

}

pub trait Wallet
{

    fn get_public_key(&self) -> [u8; PUB_KEY_LEN];

    fn get_address(&self) -> Hash
    {
        let mut hasher = Sha256::default();
        hasher.update(&self.get_public_key());

        let hash = hasher.finalize();
        *slice_as_array!(&hash, [u8; HASH_LEN]).unwrap()
    }

    fn get_status(&self, chain: &mut BlockChain) -> WalletStatus
        where Self: Sized
    {
        chain.get_wallet_status(&self.get_address())
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::private_wallet::PrivateWallet;
    use crate::logger::{Logger, LoggerLevel};
    use crate::block::Block;
    use crate::transaction::Transaction;
    use crate::chain::BlockChain;
    use crate::miner;
    use std::path::PathBuf;

    #[test]
    fn test_wallet()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::open_temp(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();

        let block_a = miner::mine_block(Block::new(&mut chain, &wallet).expect("Create block"));
        chain.add(&block_a, &mut logger).unwrap();

        let block_b = miner::mine_block(Block::new(&mut chain, &other).expect("Create block"));
        chain.add(&block_b, &mut logger).unwrap();
        
        let mut block_c = Block::new(&mut chain, &wallet).expect("Create block");
        block_c.add_transaction(Transaction::new(1, &wallet, other.get_address(), 4.6, 0.2));
        block_c.add_transaction(Transaction::new(1, &other, wallet.get_address(), 1.4, 0.2));
        block_c = miner::mine_block(block_c);
        chain.add(&block_c, &mut logger).unwrap();

        let wallet_status = wallet.get_status(&mut chain);
        assert_eq!(wallet_status.balance, block_a.calculate_reward() + block_c.calculate_reward() - 4.6 - 0.2 + 1.4 + 0.2 + 0.2);
        assert_eq!(wallet_status.max_id, 1);

        let other_status = other.get_status(&mut chain);
        assert_eq!(other_status.balance, block_b.calculate_reward() + (4.6 - 1.4 - 0.2));
        assert_eq!(other_status.max_id, 1);
    }

}

