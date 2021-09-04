use super::{Block, HASH_LEN};
use crate::wallet::WalletStatus;

use std::collections::HashSet;

impl Block
{

    pub fn get_addresses_used(&self) -> Vec<[u8; HASH_LEN]>
    {
        let mut addresses_in_use = HashSet::<[u8; HASH_LEN]>::new();
        addresses_in_use.insert(self.raward_to);
        
        for transaction in &self.transfers
        {
            addresses_in_use.insert(transaction.get_from_address());
            addresses_in_use.insert(transaction.header.to);
        }

        addresses_in_use.into_iter().collect::<Vec<_>>()
    }

    pub fn update_wallet_status(&self, address: &[u8; HASH_LEN], 
                                mut status: WalletStatus) -> Option<WalletStatus>
    {
        if &self.raward_to == address {
            status.balance += self.calculate_reward()
        }

        for transfer in &self.transfers
        {
            let header = &transfer.header;
            if &transfer.get_from_address() == address
            {
                status.balance -= header.amount + header.fee;
                if header.id <= status.max_id {
                    return None;
                }
                status.max_id = header.id;
            }

            if &header.to == address {
                status.balance += header.amount;
            }

            if &self.raward_to == address {
                status.balance += header.fee;
            }
        }

        // TODO: Pages
        Some(status)
    }

}
