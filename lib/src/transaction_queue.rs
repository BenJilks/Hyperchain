/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::transaction::{Transaction, TransactionContent};
use crate::wallet::WalletStatus;
use crate::hash::Hash;

use serde::Serialize;
use std::error::Error;

pub struct TransactionQueue<C>
    where C: TransactionContent
{
    queue: Vec<(f32, Transaction<C>)>,
}

pub fn is_depenency<C>(transaction: &Transaction<C>, depencency: &Transaction<C>) -> bool
    where C: TransactionContent + Serialize
{
    let addreses_we_use = transaction.get_addresses_used();
    let addreses_dependancy_use = depencency.get_addresses_used();

    let does_affects_us = addreses_we_use
        .iter()
        .any(|x| addreses_dependancy_use.contains(x));
    
    does_affects_us && transaction.get_id() > depencency.get_id()
}

impl<C> TransactionQueue<C>
    where C: TransactionContent + Serialize + Clone + PartialEq
{

    pub fn new() -> Self
    {
        Self
        {
            queue: Vec::new(),
        }
    }

    pub fn transactions(&self) -> impl Iterator<Item = &Transaction<C>>
    {
        self.queue
            .iter()
            .map(|(_, x)| x)
    }

    fn find_position_for_transaction(&self, new_priority: f32, new_transaction: &Transaction<C>)
        -> usize
    {
        let mut position_after_next_best_priority = None;
        let mut position_after_last_dependancy = 0;
        for (i, (priority, transaction)) in self.queue.iter().enumerate()
        {
            println!("{}-{}", *priority, new_priority);
            if position_after_next_best_priority.is_none() 
                && *priority < new_priority
            {
                position_after_next_best_priority = Some(i);
            }

            if is_depenency(&new_transaction, transaction) {
                position_after_last_dependancy = i + 1;
            }
        }

        std::cmp::max(
            position_after_next_best_priority.unwrap_or(0), 
            position_after_last_dependancy)
    }

    pub fn push(&mut self, transaction: Transaction<C>) 
        -> Result<(), Box<dyn Error>>
    {
        let priority = transaction.fee_per_byte()?;
        let position = self.find_position_for_transaction(priority, &transaction);
        println!("pos: {}", position);

        if position == self.queue.len() {
            self.queue.push((priority, transaction));
        } else {
            self.queue.insert(position, (priority, transaction));
        }
        
        Ok(())
    }

    pub fn get_next(&self, count: usize) -> impl Iterator<Item = &Transaction<C>>
    {
        let real_count = std::cmp::min(count, self.queue.len());
        self.queue[0..real_count]
            .iter()
            .map(|(_, x)| x)
    }

    pub fn remove_in_block(&mut self, transactions: &[Transaction<C>])
    {
        for transaction in transactions
        {
            let index = self.queue.iter().position(|(_, x)| x == transaction);
            if index.is_some() {
                self.queue.remove(index.unwrap());
            }
        }
    }

    pub fn remove_from_address(&mut self, address: &Hash)
    {
        self.queue
           .iter_mut()
           .take_while(|(_, x)| x.get_from_addresses().contains(&address))
           .count();
    }

    pub fn update_wallet_status(&self, address: &Hash, mut status: WalletStatus) 
        -> Result<WalletStatus, Box<dyn Error>>
    {
        for (_, transaction) in &self.queue {
            status = transaction.update_wallet_status(address, status, false)?;
        }
        Ok(status)
    }

    pub fn find(&self, transaction_id: &Hash) 
        -> Option<Transaction<C>>
    {
        for (_, transaction) in &self.queue
        {
            if let Ok(hash) = transaction.hash()
            {
                if &hash == transaction_id {
                    return Some(transaction.clone());
                }
            }
        }

        None
    }

}

