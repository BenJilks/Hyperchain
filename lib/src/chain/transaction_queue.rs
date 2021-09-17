use super::BlockChain;
use crate::block::Block;
use crate::transaction::{Transaction, TransactionVariant};
use crate::transaction::{TransactionHeader, TransactionValidationResult};
use crate::transaction::transfer::Transfer;
use crate::transaction::page::Page;
use crate::wallet::{Wallet, WalletStatus};
use crate::wallet::private_wallet::PrivateWallet;
use crate::data_store::DataUnit;
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

    pub fn new_transfer(&mut self, from: &PrivateWallet, to: Hash, amount: f32, fee: f32)
        -> Result<Option<Transaction<Transfer>>, Box<dyn Error>>
    {
        let status = self.get_wallet_status_after_queue(&from.get_address());
        self.new_transaction(from, Transfer::new(status.max_id + 1, to, amount, fee))
    }

    pub fn new_page(&mut self, from: &PrivateWallet, data: &DataUnit, fee: f32)
        -> Result<Option<Transaction<Page>>, Box<dyn Error>>
    {
        let status = self.get_wallet_status_after_queue(&from.get_address());
        self.new_transaction(from, Page::new_from_data(status.max_id + 1, data, fee)?)
    }

    fn is_transaction_valid<H>(&mut self, transaction: &Transaction<H>) -> bool
        where H: TransactionHeader + Serialize
    {
        // NOTE: We validate before adding, as everything in the transaction 
        //       queue is assumed to be valid.

        let address = transaction.get_from_address();
        let status = self.get_wallet_status_after_queue(&address);

        let new_status = transaction.update_wallet_status(&address, status, false);
        if new_status.is_none() {
            return false;
        }

        if new_status.unwrap().balance < 0.0 {
            return false;
        }

        return true;
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

#[cfg(test)]
mod tests
{

    use super::*;
    use super::super::BlockChainAddResult;
    use std::path::PathBuf;

    use crate::block::builder::BlockBuilder;
    use crate::miner;
    use crate::config::HASH_LEN;

    #[test]
    fn test_transaction_queue()
    {
        let _ = pretty_env_logger::try_init();

        let mut chain = BlockChain::open_temp();
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();

        let block_a = miner::mine_block(Block::new_blank(&mut chain, &wallet).unwrap());
        assert_eq!(chain.add(&block_a).unwrap(), BlockChainAddResult::Ok);

        let transaction_a = chain.new_transfer(&wallet, other.get_address(), 2.0, 1.0).unwrap().unwrap();
        assert_eq!(chain.push_transfer_queue(transaction_a.clone()), true);

        let transaction_b = chain.new_transfer(&wallet, other.get_address(), 3.0, 1.0).unwrap().unwrap();
        assert_eq!(chain.push_transfer_queue(transaction_b.clone()), true);

        let transaction_c = chain.new_transfer(&wallet, other.get_address(), 10.0, 1.0).unwrap().unwrap();
        assert_eq!(chain.push_transfer_queue(transaction_c), false);

        assert_eq!(chain.get_next_transfers_in_queue(10), [&transaction_a, &transaction_b]);

        let transaction_a_id_vec = transaction_a.hash().unwrap();
        let transaction_a_id = slice_as_array!(&transaction_a_id_vec, [u8; HASH_LEN]);
        assert_eq!(chain.find_transaction_in_queue(transaction_a_id.unwrap()),
                   Some(TransactionVariant::Transfer(transaction_a.clone())));

        let block_b = miner::mine_block(BlockBuilder::new(&wallet)
            .add_transfer(transaction_a)
            .add_transfer(transaction_b)
            .build(&mut chain)
            .unwrap());
        assert_eq!(chain.add(&block_b).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain.get_next_transfers_in_queue(10).is_empty(), true);
    }

}
