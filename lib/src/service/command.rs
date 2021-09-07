use crate::wallet::WalletStatus;
use crate::transaction::{Transaction, TransactionVariant};
use crate::transaction::page::Page;
use crate::block::Block;
use crate::data_store::DataUnit;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command
{
    Exit,
    Balance(Vec<u8>),
    Send(Vec<u8>, Vec<u8>, f32, f32),
    UpdatePage(Vec<u8>, String, Vec<u8>),
    TransactionInfo(Vec<u8>),
    TransactionHistory(Vec<u8>),
    Blocks(u64, u64),
    PageUpdates(Vec<u8>),
    PageData(Vec<u8>),
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
    Failed,
}
