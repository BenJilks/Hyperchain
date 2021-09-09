use super::BlockChain;
use crate::transaction::{Transaction, TransactionHeader, TransactionVariant};
use crate::transaction::page::Page;
use crate::block::Block;
use crate::wallet::WalletStatus;
use crate::config::Hash;

use serde::Serialize;

fn find_transaction<H>(transactions: &Vec<Transaction<H>>, transaction_id: &Hash)
        -> Option<Transaction<H>>
    where H: TransactionHeader + Serialize + Clone
{
    for transaction in transactions
    {
        match transaction.hash()
        {
            Ok(hash) =>
            {
                if hash == transaction_id {
                    return Some(transaction.clone());
                }
            },
            Err(_) => {},
        }
    }

    None
}

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

    pub fn last_page_update(&mut self, address: &Hash) -> Option<Block>
    {
        for block_id in (0..self.blocks.next_top()).rev()
        {
            let metadata = self.metadata.get(block_id).unwrap();
            if metadata.page_updates.contains_key(address) {
                return self.blocks.get(block_id);
            }
        }

        None
    }

    pub fn get_page_updates(&mut self, address: &Hash) 
        -> Vec<Transaction<Page>>
    {
        // FIXME: Extremely slow, need to use metadata to 
        //        optimise this!

        let mut updates = Vec::new();
        for block_id in (0..self.blocks.next_top()).rev()
        {
            let metadata = self.metadata.get(block_id).unwrap();
            if !metadata.page_updates.contains_key(address) {
                continue;
            }

            let block = self.block(block_id).unwrap();
            for page in block.pages.iter().rev() 
            {
                if &page.get_from_address() == address {
                    updates.push(page.clone());
                }
            }

            if metadata.page_updates[address].is_creation {
                break;
            }
        }

        updates.reverse();
        updates
    }

    pub fn find_transaction_in_chain(&mut self, transaction_id: &Hash) 
        -> Option<(TransactionVariant, Block)>
    {
        // FIXME: Extremely slow, need to use metadata to 
        //        optimise this!

        for block_id in 0..self.blocks.next_top() 
        {
            let block = self.block(block_id).unwrap();

            let transfer = find_transaction(&block.transfers, transaction_id);
            if transfer.is_some() {
                return Some((TransactionVariant::Transfer(transfer.unwrap()), block.clone()));
            }

            let page = find_transaction(&block.pages, transaction_id);
            if page.is_some() {
                return Some((TransactionVariant::Page(page.unwrap()), block.clone()));
            }
        }

        None
    }

    pub fn get_transaction_history(&mut self, address: &Hash) 
        -> Vec<(TransactionVariant, Option<Block>)>
    {
        // FIXME: Extremely slow, need to use metadata to 
        //        optimise this!

        let mut transactions = Vec::<(TransactionVariant, Option<Block>)>::new();
        self.walk(&mut |block|
        {
            for transfer in &block.transfers 
            {
                if &transfer.header.to == address ||
                    &transfer.get_from_address() == address
                {
                    transactions.push((
                        TransactionVariant::Transfer(transfer.clone()), 
                        Some(block.clone())));
                }
            }

            for page in &block.pages
            {
                if &page.get_from_address() == address
                {
                    transactions.push((
                        TransactionVariant::Page(page.clone()), 
                        Some(block.clone())));
                }
            }
        });

        for transfer in &self.transfer_queue 
        {
            if &transfer.header.to == address ||
                &transfer.get_from_address() == address
            {
                transactions.push((
                    TransactionVariant::Transfer(transfer.clone()), 
                    None));
            }
        }

        for page in &self.page_queue
        {
            if &page.get_from_address() == address
            {
                transactions.push((
                    TransactionVariant::Page(page.clone()), 
                    None));
            }
        }

        transactions.reverse();
        transactions
    }

}

