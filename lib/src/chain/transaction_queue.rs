use super::BlockChain;
use crate::transaction::{Transaction, TransactionValidationResult};
use crate::wallet::get_status_for_address;

use std::error::Error;

pub trait BlockChainTransactionQueue
{

    fn push_transaction_queue(&mut self, transaction: Transaction) 
        -> Result<bool, Box<dyn Error>>;

    fn get_next_transactions_in_queue(&self, count: usize) -> Vec<&Transaction>;

}

impl BlockChainTransactionQueue for BlockChain
{

    fn push_transaction_queue(&mut self, transaction: Transaction) 
        -> Result<bool, Box<dyn Error>>
    {
        if transaction.validate_content()? != TransactionValidationResult::Ok {
            return Ok(false);
        }

        let mut status = get_status_for_address(self, &transaction.get_from_address());
        status.balance -= transaction.header.amount;
        status.balance -= transaction.header.transaction_fee;
        if status.balance < 0.0 || transaction.header.id <= status.max_id {
            return Ok(false);
        }

        self.transaction_queue.push_front(transaction);
        Ok(true)
    }

    fn get_next_transactions_in_queue(&self, count: usize) -> Vec<&Transaction>
    {
        let real_count = std::cmp::min(count, self.transaction_queue.len());
        self.transaction_queue.range(0..real_count).collect()
    }

}

