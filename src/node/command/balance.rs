use super::Command;
use crate::wallet::PrivateWallet;
use crate::block::BlockChain;
use crate::node::network::NetworkConnection;
use crate::logger::Logger;

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::io::Write;

pub struct BalanceCommand;

impl Default for BalanceCommand
{

    fn default() -> Self { Self {} }

}

impl<W: Write> Command<W> for BalanceCommand
{

    fn name(&self) -> &'static str { "balance" }

    fn invoke(&mut self, args: &[String], _: &mut Arc<Mutex<NetworkConnection>>, 
        chain: &mut BlockChain, logger: &mut Logger<W>)
    {
        let wallet = PrivateWallet::read_from_file(&PathBuf::from(&args[0]), logger).unwrap();
        let balance = chain.lockup_wallet_status(&wallet).balance;
        println!("Balance: {}", balance);
    }

}
