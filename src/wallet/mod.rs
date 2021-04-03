mod private_wallet;
mod public_wallet;
pub use private_wallet::PrivateWallet;
pub use public_wallet::PublicWallet;
use crate::block::{BlockChain, Block, PUB_KEY_LEN, HASH_LEN};

use sha2::{Sha256, Digest};

pub struct WalletStatus
{
    pub balance: u32,
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

    fn calculate_status(&self, chain: &BlockChain) -> WalletStatus
    {
        let pub_key = self.get_public_key();
        let mut balance: u32 = 0;
        let mut max_id: u32 = 0;

        chain.lookup(&mut |block: &Block|
        {
            let mut is_miner = false;
            if block.raward_to == pub_key 
            {
                balance += block.calculate_reward();
                is_miner = true;
            }

            for transaction in &block.transactions
            {
                if transaction.header.to == pub_key {
                    balance += transaction.header.amount;
                }

                if transaction.header.from == pub_key 
                {
                    balance -= transaction.header.amount;
                    balance -= transaction.header.transaction_fee;
                    max_id = std::cmp::max(max_id, transaction.header.id);
                }

                if is_miner {
                    balance += transaction.header.transaction_fee;
                }
            }

            for page in &block.pages 
            {
                if page.header.site_id == pub_key {
                    balance -= page.header.page_fee;
                }

                if is_miner {
                    balance += page.header.page_fee;
                }
            }
        });

        WalletStatus
        {
            balance: balance as u32,
            max_id,
        }
    }

    fn calculate_balance(&self, chain: &BlockChain) -> u32
    {
        self.calculate_status(chain).balance
    }

}
