/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::wallet::WalletStatus;
use crate::transaction::{Transaction, TransactionVariant};
use crate::transaction::page::Page;
use crate::block::Block;
use crate::data_store::data_unit::DataUnit;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command
{
    Exit,
    Balance(Vec<u8>),
    Send(Vec<(Vec<u8>, f32)>, Vec<(Vec<u8>, f32)>, f32),
    UpdatePage(Vec<u8>, String, Vec<u8>),
    TransactionInfo(Vec<u8>),
    TransactionHistory(Vec<u8>),
    Blocks(u64, u64),
    TopBlock,
    PageUpdates(Vec<u8>),
    PageData(Vec<u8>),
    Statistics,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Statistics
{
    pub hash_rate: f64,
    pub known_chunks: usize,
    pub replication: f64,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Response
{
    Exit,
    WalletStatus(WalletStatus),
    Sent(Vec<u8>),
    TransactionInfo(TransactionVariant, Option<Block>),
    TransactionHistory(Vec<(TransactionVariant, Option<Block>)>),
    Blocks(Vec<Block>),
    PageUpdates(Vec<Transaction<Page>>),
    PageData(DataUnit),
    Statistics(Statistics),
    Failed,
}

