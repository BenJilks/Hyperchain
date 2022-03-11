use super::Block;
use crate::wallet::WalletStatus;
use crate::transaction::Transaction;
use crate::transaction::transfer::Transfer;
use crate::transaction::page::Page;
use crate::transaction::TransactionVariant;
use crate::merkle_tree::calculate_merkle_root;
use crate::hash::Hash;

use std::collections::HashSet;
use std::error::Error;

pub fn merkle_root_for_transactions(transfers: &Vec<Transaction<Transfer>>,
                                    pages: &Vec<Transaction<Page>>)
    -> Result<Hash, Box<dyn Error>>
{
    let mut hashes = Vec::new();
    for transfer in transfers {
        hashes.push(transfer.hash()?);
    }
    for page in pages {
        hashes.push(page.hash()?);
    }

    Ok(calculate_merkle_root(&hashes))
}

impl Block
{

    pub fn get_addresses_used(&self) -> Vec<Hash>
    {
        let mut addresses_in_use = HashSet::<Hash>::new();
        addresses_in_use.insert(self.header.raward_to);
        
        for transaction in &self.transfers
        {
            for address in transaction.get_from_addresses() {
                addresses_in_use.insert(address);
            }
            for output in &transaction.header.content.outputs {
                addresses_in_use.insert(output.to);
            }
        }

        for page in &self.pages 
        {
            for address in page.get_from_addresses() {
                addresses_in_use.insert(address);
            }
        }

        addresses_in_use.into_iter().collect::<Vec<_>>()
    }

    pub fn update_wallet_status(&self, address: &Hash, mut status: WalletStatus) 
        -> Result<WalletStatus, Box<dyn Error>>
    {
        if &self.header.raward_to == address {
            status.balance += self.calculate_reward()
        }

        for transfer in &self.transfers
        {
            let is_block_winner = &self.header.raward_to == address;
            status = transfer.update_wallet_status(address, status, is_block_winner)?;
        }

        for page in &self.pages
        {
            let is_block_winner = &self.header.raward_to == address;
            status = page.update_wallet_status(address, status, is_block_winner)?;
        }

        Ok(status)
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

