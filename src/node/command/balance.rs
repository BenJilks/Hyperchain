use super::Command;
use crate::wallet::PrivateWallet;
use crate::block::BlockChain;
use crate::node::network::NetworkConnection;

use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub struct BalanceCommand;

impl Default for BalanceCommand
{

    fn default() -> Self { Self {} }

}

impl Command for BalanceCommand
{

    fn name(&self) -> &'static str { "balance" }

    fn invoke(&mut self, args: &[String], _: &mut Arc<Mutex<NetworkConnection>>, chain: &mut BlockChain)
    {
        let wallet = PrivateWallet::read_from_file(&PathBuf::from(&args[0])).unwrap();
        let balance = chain.longest_branch().lockup_wallet_status(&wallet).balance;
        println!("Balance: {}", balance);
    }

}
