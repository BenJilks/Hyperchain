use super::Block;
use crate::wallet::WalletStatus;
use crate::transaction::TransactionVariant;
use crate::config::Hash;

use std::collections::HashSet;

impl Block
{

    pub fn get_addresses_used(&self) -> Vec<Hash>
    {
        let mut addresses_in_use = HashSet::<Hash>::new();
        addresses_in_use.insert(self.raward_to);
        
        for transaction in &self.transfers
        {
            addresses_in_use.insert(transaction.get_from_address());
            addresses_in_use.insert(transaction.header.to);
        }

        for page in &self.pages {
            addresses_in_use.insert(page.get_from_address());
        }

        addresses_in_use.into_iter().collect::<Vec<_>>()
    }

    pub fn update_wallet_status(&self, address: &Hash, mut status: WalletStatus) 
        -> Option<WalletStatus>
    {
        if &self.raward_to == address {
            status.balance += self.calculate_reward()
        }

        for transfer in &self.transfers
        {
            let is_block_winner = &self.raward_to == address;
            match transfer.update_wallet_status(address, status, is_block_winner)
            {
                Some(new_status) => status = new_status,
                None => return None,
            }
        }

        for page in &self.pages
        {
            let is_block_winner = &self.raward_to == address;
            match page.update_wallet_status(address, status, is_block_winner)
            {
                Some(new_status) => status = new_status,
                None => return None,
            }
        }

        Some(status)
    }

    pub fn transactions(&self) -> Vec<TransactionVariant>
    {
        let mut transactions = Vec::new();
        for transfer in &self.transfers {
            transactions.push(TransactionVariant::Transfer(transfer.clone()));
        }
        for page in &self.pages {
            transactions.push(TransactionVariant::Page(page.clone()));
        }

        transactions
    }

}
