use crate::wallet::WalletStatus;
use crate::transaction::transfer::Transfer;
use crate::block::Block;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command
{
    Exit,
    Balance(Vec<u8>),
    Send(Vec<u8>, Vec<u8>, f32, f32),
    TransactionInfo(Vec<u8>),
    TransactionHistory(Vec<u8>),
    Blocks(u64, u64)
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Response
{
    Exit,
    WalletStatus(WalletStatus),
    Sent(Vec<u8>),
    TransactionInfo(Transfer, Option<Block>),
    TransactionHistory(Vec<(Transfer, Option<Block>)>),
    Blocks(Vec<Block>),
    Failed,
}
