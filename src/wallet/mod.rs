mod private_wallet;
mod public_wallet;
pub use private_wallet::PrivateWallet;
pub use public_wallet::PublicWallet;

use crate::block::{BlockChain, Block, PUB_KEY_LEN};

pub struct WalletStatus
{
    pub balance: u32,
    pub max_id: u32,
}

pub trait Wallet
{

    fn get_public_key(&self) -> [u8; PUB_KEY_LEN];

    fn calculate_status(&self, chain: &BlockChain) -> WalletStatus
    {
        let pub_key = self.get_public_key();
        let mut balance: i32 = 0;
        let mut max_id: u32 = 0;

        chain.lookup(&mut |block: &Block|
        {
            let mut is_miner = false;
            if block.raward_to == pub_key 
            {
                balance += block.raward as i32;
                is_miner = true;
            }

            for transaction in &block.transactions
            {
                if transaction.header.to == pub_key {
                    balance += transaction.header.amount as i32;
                }

                if transaction.header.from == pub_key 
                {
                    balance -= transaction.header.amount as i32;
                    balance -= transaction.header.transaction_fee as i32;
                    max_id = std::cmp::max(max_id, transaction.header.id);
                }

                if is_miner {
                    balance += transaction.header.transaction_fee as i32;
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
