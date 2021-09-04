use super::BlockChain;
use crate::block::Block;
use crate::transaction::{Transaction, TransactionValidationResult};
use crate::wallet::{Wallet, WalletStatus};
use crate::wallet::private_wallet::PrivateWallet;
use crate::config::Hash;

use std::error::Error;

impl BlockChain
{

    pub fn remove_from_transaction_queue(&mut self, block: &Block)
    {
        for transaction in &block.transactions
        {
            let index = self.transaction_queue.iter().position(|x| x == transaction);
            if index.is_some() {
                self.transaction_queue.remove(index.unwrap());
            }
        }
    }

    fn get_wallet_status_after_queue(&mut self, address: &Hash) -> WalletStatus
    {
        let mut status = self.get_wallet_status(address);
        for transaction in &self.transaction_queue
        {
            if &transaction.get_from_address() == address 
            {
                // NOTE: We assume everything in the queue is 
                //       valid, for now.

                status.balance -= transaction.header.amount;
                status.balance -= transaction.header.transaction_fee;
                status.max_id = transaction.header.id;
            }

            if &transaction.header.to == address {
                status.balance += transaction.header.amount;
            }
        }

        status
    }

    pub fn new_transaction(&mut self, from: &PrivateWallet, to: Hash, amount: f32, fee: f32)
        -> Result<Option<Transaction>, Box<dyn Error>>
    {
        let mut status = self.get_wallet_status_after_queue(&from.get_address());
        status.balance -= amount;
        status.balance -= fee;
        if status.balance < 0.0 {
            return Ok(None);
        }

        let transaction = Transaction::new(status.max_id + 1, from, to, amount, fee);
        if transaction.validate_content()? != TransactionValidationResult::Ok {
            return Ok(None);
        }

        Ok(Some(transaction))
    }

    pub fn push_transaction_queue(&mut self, transaction: Transaction) -> bool
    {
        // NOTE: We validate before adding, as everything in the transaction 
        //       queue is assumed to be valid.
        let mut status = self.get_wallet_status_after_queue(&transaction.get_from_address());
        status.balance -= transaction.header.amount;
        status.balance -= transaction.header.transaction_fee;
        if status.balance < 0.0 || transaction.header.id <= status.max_id {
            return false;
        }

        self.transaction_queue.push_back(transaction);
        true
    }

    pub fn get_next_transactions_in_queue(&self, count: usize) -> Vec<&Transaction>
    {
        let real_count = std::cmp::min(count, self.transaction_queue.len());
        self.transaction_queue.range(0..real_count).collect()
    }

    pub fn find_transaction_in_queue(&self, transaction_id: &Hash) -> Option<Transaction>
    {
        for transaction in &self.transaction_queue
        {
            match transaction.header.hash()
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

}
