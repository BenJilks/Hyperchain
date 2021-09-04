use super::BlockChain;
use crate::block::Block;
use crate::transaction::transfer::{Transfer, TransferValidationResult};
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
        -> Result<Option<Transfer>, Box<dyn Error>>
    {
        let mut status = self.get_wallet_status_after_queue(&from.get_address());
        status.balance -= amount;
        status.balance -= fee;
        if status.balance < 0.0 {
            return Ok(None);
        }

        let transfer = Transfer::new(status.max_id + 1, from, to, amount, fee);
        if transfer.validate_content()? != TransferValidationResult::Ok {
            return Ok(None);
        }

        Ok(Some(transfer))
    }

    pub fn push_transaction_queue(&mut self, transfer: Transfer) -> bool
    {
        // NOTE: We validate before adding, as everything in the transaction 
        //       queue is assumed to be valid.
        let mut status = self.get_wallet_status_after_queue(&transfer.get_from_address());
        status.balance -= transfer.header.amount;
        status.balance -= transfer.header.fee;
        if status.balance < 0.0 || transfer.header.id <= status.max_id {
            return false;
        }

        self.transfer_queue.push_back(transfer);
        true
    }

    pub fn get_next_transactions_in_queue(&self, count: usize) -> Vec<&Transfer>
    {
        let real_count = std::cmp::min(count, self.transfer_queue.len());
        self.transfer_queue.range(0..real_count).collect()
    }

    pub fn find_transaction_in_queue(&self, transfer_id: &Hash) -> Option<Transfer>
    {
        for transfer in &self.transfer_queue
        {
            match transfer.header.hash()
            {
                Ok(hash) =>
                {
                    if hash == transfer_id {
                        return Some(transfer.clone());
                    }
                },
                Err(_) => {},
            }
        }

        None
    }

}
