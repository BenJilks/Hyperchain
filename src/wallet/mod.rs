mod private_wallet;
mod public_wallet;
pub use private_wallet::PrivateWallet;
pub use public_wallet::PublicWallet;
use crate::block::{PUB_KEY_LEN, HASH_LEN};
use crate::block::BlockChain;

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WalletStatus
{
    pub balance: f32,
    pub max_id: u32,
}

pub trait Wallet
{

    fn get_public_key(&self) -> [u8; PUB_KEY_LEN];

    fn get_address(&self) -> [u8; HASH_LEN]
    {
        let mut hasher = Sha256::default();
        hasher.update(&self.get_public_key());

        let hash = hasher.finalize();
        *slice_as_array!(&hash, [u8; HASH_LEN]).unwrap()
    }

    fn get_status(&self, chain: &BlockChain) -> WalletStatus
    {
        let mut status = WalletStatus
        {
            balance: 0.0,
            max_id: 0u32,
        };

        let public_key = self.get_public_key();
        let address = self.get_address();
        chain.walk_chain(&mut |block|
        {
            if block.raward_to == address {
                status.balance += block.calculate_reward()
            }

            for transaction in &block.transactions
            {
                let header = &transaction.header;
                if header.from == public_key 
                {
                    status.balance -= header.amount + header.transaction_fee;
                    status.max_id = std::cmp::max(status.max_id, header.id);
                }

                if header.to == address {
                    status.balance += header.amount;
                }

                if block.raward_to == address {
                    status.balance += header.transaction_fee;
                }
            }

            // TODO: Pages
        });

        status
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::logger::{Logger, LoggerLevel};
    use crate::block::Block;
    use crate::block::Transaction;
    use crate::miner;
    use std::path::PathBuf;

    #[test]
    fn test_wallet()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::new(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();

        let block_a = miner::mine_block(Block::new(&mut chain, &wallet).expect("Create block"));
        chain.add(&block_a, &mut logger);

        let block_b = miner::mine_block(Block::new(&mut chain, &other).expect("Create block"));
        chain.add(&block_b, &mut logger);
        
        let mut block_c = Block::new(&mut chain, &wallet).expect("Create block");
        block_c.add_transaction(Transaction::for_block(&chain, &wallet, &other, 4.6, 0.2).expect("Create transaction"));
        block_c.add_transaction(Transaction::for_block(&chain, &other, &wallet, 1.4, 0.2).expect("Create transaction"));
        block_c = miner::mine_block(block_c);
        chain.add(&block_c, &mut logger);

        let wallet_status = wallet.get_status(&chain);
        assert_eq!(wallet_status.balance, block_a.calculate_reward() + block_c.calculate_reward() - 4.6 - 0.2 + 1.4 + 0.2 + 0.2);
        assert_eq!(wallet_status.max_id, 1);

        let other_status = other.get_status(&chain);
        assert_eq!(other_status.balance, block_b.calculate_reward() + (4.6 - 1.4 - 0.2));
        assert_eq!(other_status.max_id, 1);
    }

}
