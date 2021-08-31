use crate::wallet::WalletStatus;
use crate::wallet::public_wallet::PublicWallet;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Command
{
    Exit,
    Balance(PublicWallet),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Response
{
    Exit,
    WalletStatus(WalletStatus),
    Ok,
}

