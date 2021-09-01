use super::BlockChain;
use crate::block::Hash;
use crate::transaction::{Transaction, TransactionValidationResult};

use std::error::Error;

impl BlockChain
{

    pub fn push_transaction_queue(&mut self, transaction: Transaction) 
        -> Result<bool, Box<dyn Error>>
    {
        if transaction.validate_content()? != TransactionValidationResult::Ok {
            return Ok(false);
        }

        let mut status = self.get_wallet_status(&transaction.get_from_address());
        status.balance -= transaction.header.amount;
        status.balance -= transaction.header.transaction_fee;
        if status.balance < 0.0 || transaction.header.id <= status.max_id {
            return Ok(false);
        }

        self.transaction_queue.push_front(transaction);
        Ok(true)
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

