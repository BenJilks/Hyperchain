use crate::wallet::WalletStatus;
use crate::wallet::public_wallet::PublicWallet;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command
{
    Exit,
    Balance(PublicWallet),
    Send(Vec<u8>, Vec<u8>, f32, f32),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Response
{
    Exit,
    WalletStatus(WalletStatus),
    Sent(Vec<u8>),
    Failed,
}

