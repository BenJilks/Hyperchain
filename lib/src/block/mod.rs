/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

pub mod validate;
pub mod target;
pub mod builder;
mod transactions;
use target::{calculate_target, Target};
use transactions::merkle_root_for_transactions;
use crate::transaction::Transaction;
use crate::transaction::transfer::Transfer;
use crate::transaction::page::Page;
use crate::chain::BlockChain;
use crate::wallet::Wallet;
use crate::config::HASH_LEN;
use crate::hash::Hash;

use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use std::error::Error;
use bincode;

pub fn current_timestamp() -> u128
{
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_millis()
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct BlockHeader
{
    pub prev_hash: Hash,
    pub block_id: u64,
    pub timestamp: u128,
    pub raward_to: Hash,
    pub target: Target,
    pub transaction_merkle_root: Hash,
    pub pow: u64, // TODO: This should be a correct size
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Block
{
    pub header: BlockHeader,
    pub pages: Vec<Transaction<Page>>,
    pub transfers: Vec<Transaction<Transfer>>,
}

impl std::fmt::Debug for Block
{

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
    {
        f.debug_struct("Block")
            .field("block_id", &self.header.block_id)
            .field("timestamp", &self.header.timestamp)
            .field("target", &self.header.target)
            .field("pow", &self.header.pow)
            .finish()
    }

}

impl Block
{

    pub fn new_blank<W: Wallet>(chain: &mut BlockChain, raward_to: &W)
        -> Result<Self, Box<dyn Error>>
    {
        Self::new(chain, raward_to, Vec::new(), Vec::new())
    }

    pub fn new<W: Wallet>(chain: &mut BlockChain, raward_to: &W, 
                          transfers: Vec<Transaction<Transfer>>,
                          pages: Vec<Transaction<Page>>)
        -> Result<Self, Box<dyn Error>>
    {
        let (sample_start, sample_end) = chain.take_sample();
        let target = calculate_target(sample_start, sample_end);
        let (prev_block_id, prev_hash) =
            match chain.top()
            {
                Some(top) => (Some(top.header.block_id), top.hash()?),
                None => (None, Hash::empty()),
            };

        let block_id =
            match prev_block_id
            {
                Some(id) => id + 1,
                None => 0,
            };

        let timestamp = current_timestamp();
        let transaction_merkle_root = merkle_root_for_transactions(&transfers, &pages)?;
        Ok(Block
        {
            header: BlockHeader
            {
                prev_hash,
                block_id,
                timestamp,
                raward_to: raward_to.get_address(),
                target,
                transaction_merkle_root,
                pow: 0,
            },

            pages,
            transfers,
        })
    }

    pub fn calculate_reward(&self) -> f32
    {
        // TODO: do real reward calc
        10.0
    }

    pub fn hash(&self) -> Result<Hash, Box<dyn Error>>
    {
        let mut hasher = Sha256::default();
        let bytes = bincode::serialize(&self.header)?;
        hasher.update(&bytes);
        Ok(Hash::from(&hasher.clone().finalize()))
    }

}

