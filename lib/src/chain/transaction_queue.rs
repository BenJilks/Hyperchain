/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use super::BlockChain;
use crate::block::Block;
use crate::transaction::{Transaction, TransactionVariant};
use crate::transaction::{TransactionContent, TransactionValidationResult};
use crate::transaction::transfer::{Transfer, TransferBuilder};
use crate::transaction::page::Page;
use crate::transaction::builder::TransactionBuilder;
use crate::wallet::{Wallet, WalletStatus};
use crate::wallet::private_wallet::PrivateWallet;
use crate::data_store::data_unit::DataUnit;
use crate::error::ErrorMessage;
use crate::hash::Hash;

use serde::Serialize;
use std::error::Error;

impl BlockChain
{

    fn get_wallet_status_after_queue(&mut self, address: &Hash) -> WalletStatus
    {
        let mut status = self.get_wallet_status(address);
        status = self.transfer_queue.update_wallet_status(address, status).unwrap();
        status = self.page_queue.update_wallet_status(address, status).unwrap();
        status
    }

    fn new_transaction<C>(&mut self, inputs: Vec<(&PrivateWallet, f32)>, content: C)
            -> Result<Transaction<C>, Box<dyn Error>>
        where C: TransactionContent + Serialize
    {
        let mut builder = TransactionBuilder::new(content);
        for (wallet, amount) in &inputs {
            builder = builder.add_input(wallet, *amount);
        }

        let transaction = builder.build()?;
        if transaction.validate_content()? != TransactionValidationResult::Ok {
            return Err(ErrorMessage::new("Invalid content"));
        }

        for (wallet, _) in inputs
        {
            let status = self.get_wallet_status_after_queue(&wallet.get_address());
            transaction.update_wallet_status(&wallet.get_address(), status, false)?;
        }

        Ok(transaction)
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

    pub fn new_transfer(&mut self, 
                        inputs: Vec<(&PrivateWallet, f32)>, 
                        outputs: Vec<(Hash, f32)>, 
                        fee: f32)
        -> Result<Transaction<Transfer>, Box<dyn Error>>
    {
        let id = self.next_transaction_id(&inputs);
        let mut transfer_builder = TransferBuilder::new(id, fee);
        for (to, amount) in outputs {
            transfer_builder = transfer_builder.add_output(to, amount);
        }

        self.new_transaction(inputs, transfer_builder.build())
    }

    pub fn new_page(&mut self, from: &PrivateWallet, data: &DataUnit, fee: f32)
        -> Result<Transaction<Page>, Box<dyn Error>>
    {
        let status = self.get_wallet_status_after_queue(&from.get_address());
        let page = Page::new_from_data(status.max_id + 1, from.get_address(), data, fee)?;
        let total_output = page.cost() + fee;
        self.new_transaction(vec![(from, total_output)], page)
    }

    fn is_transaction_valid<C>(&mut self, transaction: &Transaction<C>) -> Result<(), Box<dyn Error>>
        where C: TransactionContent + Serialize
    {
        // NOTE: We validate before adding, as everything in the transaction 
        //       queue is assumed to be valid.

        for address in transaction.get_from_addresses()
        {
            let status = self.get_wallet_status_after_queue(&address);

            let new_status = transaction.update_wallet_status(&address, status, false)?;
            if new_status.balance < 0.0 {
                return Err(ErrorMessage::new("Negative balance"));
            }
        }

        Ok(())
    }

    pub fn push_transfer_queue(&mut self, transaction: Transaction<Transfer>) 
        -> Result<(), Box<dyn Error>>
    {
        self.is_transaction_valid(&transaction)?;
        self.transfer_queue.push(transaction)?;
        Ok(())
    }

    pub fn push_page_queue(&mut self, transaction: Transaction<Page>) 
        -> Result<(), Box<dyn Error>>
    {
        self.is_transaction_valid(&transaction)?;
        self.page_queue.push(transaction)?;
        Ok(())
    }

    pub fn get_next_transfers_in_queue(&self, count: usize) 
        -> impl Iterator<Item = &Transaction<Transfer>>
    {
        self.transfer_queue.get_next(count)
    }

    pub fn get_next_pages_in_queue(&self, count: usize) 
        -> impl Iterator<Item = &Transaction<Page>>
    {
        self.page_queue.get_next(count)
    }

    pub fn remove_from_transaction_queue(&mut self, block: &Block)
    {
        self.transfer_queue.remove_in_block(&block.transfers);
        self.page_queue.remove_in_block(&block.pages);
    }

    pub fn find_transaction_in_queue(&self, transaction_id: &Hash) -> Option<TransactionVariant>
    {
        let transfer = self.transfer_queue.find(transaction_id);
        if transfer.is_some() {
            return Some(TransactionVariant::Transfer(transfer?));
        }

        let page = self.page_queue.find(transaction_id);
        if page.is_some() {
            return Some(TransactionVariant::Page(page?));
        }

        None
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::super::BlockChainAddResult;

    use crate::block::builder::BlockBuilder;
    use crate::miner;

    #[test]
    fn test_transaction_queue()
    {
        let _ = pretty_env_logger::try_init();

        let mut chain = BlockChain::open_temp();
        let wallet = PrivateWallet::open_temp(0).unwrap();
        let other = PrivateWallet::open_temp(1).unwrap();
        let independant_a = PrivateWallet::open_temp(2).unwrap();
        let independant_b = PrivateWallet::open_temp(3).unwrap();

        let block_a = miner::mine_block(Block::new_blank(&mut chain, &wallet).unwrap());
        assert_eq!(chain.add(&block_a).unwrap(), BlockChainAddResult::Ok);

        let block_b = miner::mine_block(Block::new_blank(&mut chain, &independant_a).unwrap());
        assert_eq!(chain.add(&block_b).unwrap(), BlockChainAddResult::Ok);

        let transaction_a = chain.new_transfer(
            vec![(&wallet, 3.0)], 
            vec![(other.get_address(), 2.0)],
            1.0)
            .unwrap();
        chain.push_transfer_queue(transaction_a.clone()).unwrap();

        let transaction_b = chain.new_transfer(
            vec![(&wallet, 3.0)], 
            vec![(other.get_address(), 1.0)], 
            2.0)
            .unwrap();
        chain.push_transfer_queue(transaction_b.clone()).unwrap();

        let transaction_c = chain.new_transfer(
            vec![(&wallet, 11.0)], 
            vec![(other.get_address(), 10.0)], 
            1.0)
            .unwrap();
        assert_eq!(chain.push_transfer_queue(transaction_c).is_err(), true);

        let transaction_d = chain.new_transfer(
            vec![(&independant_a, 6.0)], 
            vec![(independant_b.get_address(), 3.0)], 
            3.0)
            .unwrap();
        chain.push_transfer_queue(transaction_d.clone()).unwrap();

        assert_eq!(
            chain.get_next_transfers_in_queue(10).collect::<Vec<_>>(), 
            [&transaction_d, &transaction_a, &transaction_b]);

        let transaction_a_id = transaction_a.hash().unwrap();
        assert_eq!(chain.find_transaction_in_queue(&transaction_a_id),
                   Some(TransactionVariant::Transfer(transaction_a.clone())));

        let block_b = miner::mine_block(BlockBuilder::new(&wallet)
            .add_transfer(transaction_a)
            .add_transfer(transaction_b)
            .add_transfer(transaction_d)
            .build(&mut chain)
            .unwrap());
        assert_eq!(chain.add(&block_b).unwrap(), BlockChainAddResult::Ok);
        assert_eq!(chain.get_next_transfers_in_queue(10).count() == 0, true);
    }

}

