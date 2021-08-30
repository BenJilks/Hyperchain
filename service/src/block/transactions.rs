use super::{Block, HASH_LEN};
use crate::wallet::WalletStatus;

use std::collections::HashSet;

pub trait BlockTransactions
{
    fn get_addresses_used(&self) -> Vec<[u8; HASH_LEN]>;
    fn update_wallet_status(&self, address: &[u8; HASH_LEN], status: &mut WalletStatus);
}

impl BlockTransactions for Block
{

    fn get_addresses_used(&self) -> Vec<[u8; HASH_LEN]>
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

    fn update_wallet_status(&self, address: &[u8; HASH_LEN], status: &mut WalletStatus)
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

