use super::BlockChain;
use crate::block::Block;
use crate::transaction::{Transaction, TransactionValidationResult};
use crate::transaction::transfer::Transfer;
use crate::wallet::{Wallet, WalletStatus};
use crate::wallet::private_wallet::PrivateWallet;
use crate::config::Hash;

use std::error::Error;

impl BlockChain
{

    pub fn remove_from_transaction_queue(&mut self, block: &Block)
    {
        for transfer in &block.transfers
        {
            let index = self.transfer_queue.iter().position(|x| x == transfer);
            if index.is_some() {
                self.transfer_queue.remove(index.unwrap());
            }
        }
    }

    fn get_wallet_status_after_queue(&mut self, address: &Hash) -> WalletStatus
    {
        let mut status = self.get_wallet_status(address);
        for transfer in &self.transfer_queue
        {
            if &transfer.get_from_address() == address 
            {
                // NOTE: We assume everything in the queue is 
                //       valid, for now.

                status.balance -= transfer.header.amount;
                status.balance -= transfer.header.fee;
                status.max_id = transfer.header.id;
            }

            if &transfer.header.to == address {
                status.balance += transfer.header.amount;
            }
        }

        status
    }

    pub fn new_transfer(&mut self, from: &PrivateWallet, to: Hash, amount: f32, fee: f32)
        -> Result<Option<Transaction<Transfer>>, Box<dyn Error>>
    {
        let mut status = self.get_wallet_status_after_queue(&from.get_address());
        status.balance -= amount;
        status.balance -= fee;
        if status.balance < 0.0 {
            return Ok(None);
        }

        let transaction = Transaction::new(from, Transfer::new(status.max_id + 1, to, amount, fee));
        if transaction.validate_content()? != TransactionValidationResult::Ok {
            return Ok(None);
        }

        Ok(Some(transaction))
    }

    pub fn push_transaction_queue(&mut self, transaction: Transaction<Transfer>) -> bool
    {
        // NOTE: We validate before adding, as everything in the transaction 
        //       queue is assumed to be valid.
        let mut status = self.get_wallet_status_after_queue(&transaction.get_from_address());
        status.balance -= transaction.header.amount;
        status.balance -= transaction.header.fee;
        if status.balance < 0.0 || transaction.header.id <= status.max_id {
            return false;
        }

        self.transfer_queue.push_back(transaction);
        true
    }

    pub fn get_next_transactions_in_queue(&self, count: usize) -> Vec<&Transaction<Transfer>>
    {
        let real_count = std::cmp::min(count, self.transfer_queue.len());
        self.transfer_queue.range(0..real_count).collect()
    }

    pub fn find_transaction_in_queue(&self, transaction_id: &Hash) -> Option<Transaction<Transfer>>
    {
        for transaction in &self.transfer_queue
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

}
