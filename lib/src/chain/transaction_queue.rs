use super::BlockChain;
use crate::block::Block;
use crate::transaction::{Transaction, TransactionVariant};
use crate::transaction::{TransactionHeader, TransactionValidationResult};
use crate::transaction::transfer::Transfer;
use crate::transaction::page::Page;
use crate::wallet::{Wallet, WalletStatus};
use crate::wallet::private_wallet::PrivateWallet;
use crate::config::Hash;

use serde::Serialize;
use std::error::Error;
use std::collections::VecDeque;

fn search_queue<H>(queue: &VecDeque<Transaction<H>>, transaction_id: &Hash) 
        -> Option<Transaction<H>>
    where H: TransactionHeader + Serialize + Clone
{
    for transaction in queue
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

    fn get_wallet_status_after_queue(&mut self, address: &Hash) -> WalletStatus
    {
        let mut status = self.get_wallet_status(address);
        for transfer in &self.transfer_queue {
            status = transfer.update_wallet_status(address, status, false).unwrap();
        }
        for page in &self.page_queue {
            status = page.update_wallet_status(address, status, false).unwrap();
        }

        status
    }

    fn new_transaction<H>(&mut self, from: &PrivateWallet, header: H)
            -> Result<Option<Transaction<H>>, Box<dyn Error>>
        where H: TransactionHeader + Serialize
    {
        let transaction = Transaction::new(from, header);
        if transaction.validate_content()? != TransactionValidationResult::Ok {
            return Ok(None);
        }

        let status = self.get_wallet_status_after_queue(&from.get_address());
        if transaction.update_wallet_status(&from.get_address(), status, false).is_some() {
            Ok(Some(transaction))
        } else {
            Ok(None)
        }
    }

    fn is_transaction_valid<H>(&mut self, transaction: &Transaction<H>) -> bool
        where H: TransactionHeader + Serialize
    {
        // NOTE: We validate before adding, as everything in the transaction 
        //       queue is assumed to be valid.

        let address = transaction.get_from_address();
        let status = self.get_wallet_status_after_queue(&address);
        transaction.update_wallet_status(&address, status, false).is_some()
    }

    pub fn remove_from_transaction_queue(&mut self, block: &Block)
    {
        for transfer in &block.transfers
        {
            let index = self.transfer_queue.iter().position(|x| x == transfer);
            if index.is_some() {
                self.transfer_queue.remove(index.unwrap());
            }
        }

        for page in &block.pages
        {
            let index = self.page_queue.iter().position(|x| x == page);
            if index.is_some() {
                self.page_queue.remove(index.unwrap());
            }
        }
    }

    pub fn new_transfer(&mut self, from: &PrivateWallet, to: Hash, amount: f32, fee: f32)
        -> Result<Option<Transaction<Transfer>>, Box<dyn Error>>
    {
        let status = self.get_wallet_status_after_queue(&from.get_address());
        self.new_transaction(from, Transfer::new(status.max_id + 1, to, amount, fee))
    }

    pub fn new_page(&mut self, from: &PrivateWallet, data_hashes: Vec<Hash>, data_length: u32, fee: f32)
        -> Result<Option<Transaction<Page>>, Box<dyn Error>>
    {
        let status = self.get_wallet_status_after_queue(&from.get_address());
        self.new_transaction(from, Page::new(status.max_id + 1, data_hashes, data_length, fee))
    }

    pub fn push_transfer_queue(&mut self, transaction: Transaction<Transfer>) -> bool
    {
        if !self.is_transaction_valid(&transaction) {
            return false;
        }
        self.transfer_queue.push_back(transaction);
        true
    }

    pub fn push_page_queue(&mut self, transaction: Transaction<Page>) -> bool
    {
        if !self.is_transaction_valid(&transaction) {
            return false;
        }
        self.page_queue.push_back(transaction);
        true
    }

    pub fn get_next_transfers_in_queue(&self, count: usize) -> Vec<&Transaction<Transfer>>
    {
        let real_count = std::cmp::min(count, self.transfer_queue.len());
        self.transfer_queue.range(0..real_count).collect()
    }

    pub fn get_next_pages_in_queue(&self, count: usize) -> Vec<&Transaction<Page>>
    {
        let real_count = std::cmp::min(count, self.page_queue.len());
        self.page_queue.range(0..real_count).collect()
    }

    pub fn find_transaction_in_queue(&self, transaction_id: &Hash) -> Option<TransactionVariant>
    {
        let transfer = search_queue(&self.transfer_queue, transaction_id);
        if transfer.is_some() {
            return Some(TransactionVariant::Transfer(transfer.unwrap()));
        }

        let page = search_queue(&self.page_queue, transaction_id);
        if page.is_some() {
            return Some(TransactionVariant::Page(page.unwrap()));
        }

        None
    }

}
