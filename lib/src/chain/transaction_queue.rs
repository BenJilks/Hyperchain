use super::BlockChain;
use crate::block::Block;
use crate::transaction::{Transaction, TransactionVariant};
use crate::transaction::{TransactionContent, TransactionValidationResult};
use crate::transaction::transfer::Transfer;
use crate::transaction::page::Page;
use crate::transaction::builder::TransactionBuilder;
use crate::wallet::{Wallet, WalletStatus};
use crate::wallet::private_wallet::PrivateWallet;
use crate::data_store::DataUnit;
use crate::config::Hash;

use serde::Serialize;
use std::error::Error;
use std::collections::VecDeque;

fn search_queue<C>(queue: &VecDeque<Transaction<C>>, transaction_id: &Hash) 
        -> Option<Transaction<C>>
    where C: TransactionContent + Serialize + Clone
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

    fn new_transaction<C>(&mut self, inputs: Vec<(&PrivateWallet, f32)>, content: C)
            -> Result<Option<Transaction<C>>, Box<dyn Error>>
        where C: TransactionContent + Serialize
    {
        let mut builder = TransactionBuilder::new(content);
        for (wallet, amount) in &inputs {
            builder = builder.add_input(wallet, *amount);
        }

        let transaction = builder.build()?;
        if transaction.validate_content()? != TransactionValidationResult::Ok 
        {
            debug!("Invalid content");
            return Ok(None);
        }

        for (wallet, _) in inputs
        {
            let status = self.get_wallet_status_after_queue(&wallet.get_address());
            if transaction.update_wallet_status(&wallet.get_address(), status, false).is_none() {
                return Ok(None);
            }
        }

        Ok(Some(transaction))
    }

    fn next_transaction_id(&mut self, inputs: &Vec<(&PrivateWallet, f32)>) -> u32
    {
        let mut max_id = 0;
        for (wallet, _) in inputs
        {
            let status = self.get_wallet_status_after_queue(&wallet.get_address());
            max_id = std::cmp::max(max_id, status.max_id);
        }

        max_id + 1
    }

    pub fn new_transfer(&mut self, inputs: Vec<(&PrivateWallet, f32)>, to: Hash, amount: f32, fee: f32)
        -> Result<Option<Transaction<Transfer>>, Box<dyn Error>>
    {
        let id = self.next_transaction_id(&inputs);
        self.new_transaction(inputs, Transfer::new(id, to, amount, fee))
    }

    pub fn new_page(&mut self, from: &PrivateWallet, data: &DataUnit, fee: f32)
        -> Result<Option<Transaction<Page>>, Box<dyn Error>>
    {
        let status = self.get_wallet_status_after_queue(&from.get_address());
        let page = Page::new_from_data(status.max_id + 1, from.get_address(), data, fee)?;
        let total_output = page.cost() + fee;
        self.new_transaction(vec![(from, total_output)], page)
    }

    fn is_transaction_valid<C>(&mut self, transaction: &Transaction<C>) -> bool
        where C: TransactionContent + Serialize
    {
        // NOTE: We validate before adding, as everything in the transaction 
        //       queue is assumed to be valid.

        for address in transaction.get_from_addresses()
        {
            let status = self.get_wallet_status_after_queue(&address);

            let new_status = transaction.update_wallet_status(&address, status, false);
            if new_status.is_none() {
                return false;
            }

            if new_status.unwrap().balance < 0.0 {
                return false;
            }
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

        let transaction_a = chain.new_transfer(vec![(&wallet, 3.0)], other.get_address(), 2.0, 1.0).unwrap().unwrap();
        assert_eq!(chain.push_transfer_queue(transaction_a.clone()), true);

        let transaction_b = chain.new_transfer(vec![(&wallet, 3.0)], other.get_address(), 2.0, 1.0).unwrap().unwrap();
        assert_eq!(chain.push_transfer_queue(transaction_b.clone()), true);

        let transaction_c = chain.new_transfer(vec![(&wallet, 11.0)], other.get_address(), 10.0, 1.0).unwrap().unwrap();
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
