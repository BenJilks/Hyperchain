use super::BlockChain;
use crate::transaction::Transaction;
use crate::block::{Block, Hash};
use crate::wallet::WalletStatus;

impl BlockChain
{

    pub fn get_wallet_status_up_to_block(&mut self, to: u64, address: &Hash) -> WalletStatus
    {
        let real_to = std::cmp::min(to + 1, self.blocks.next_top());
        for block_id in (0..real_to).rev()
        {
            let metadata = self.metadata.get(block_id).unwrap();
            if metadata.wallets.contains_key(address) {
                return metadata.wallets[address].clone();
            }
        }

        WalletStatus::default()
    }

    pub fn get_wallet_status(&mut self, address: &Hash) -> WalletStatus
    {
        if self.blocks.next_top() == 0 {
            WalletStatus::default()
        } else {
            self.get_wallet_status_up_to_block(self.blocks.next_top() - 1, address)
        }
    }

    pub fn find_transaction_in_chain(&mut self, transaction_id: &Hash) 
        -> Option<(Transaction, Block)>
    {
        for block_id in 0..self.blocks.next_top() 
        {
            let block = self.block(block_id).unwrap();
            for transaction in &block.transactions
            {
                match transaction.header.hash()
                {
                    Ok(hash) =>
                    {
                        if hash == transaction_id {
                            return Some((transaction.clone(), block));
                        }
                    },
                    Err(_) => {},
                }
            }
        }

        None
    }

}
