use super::Command;
use crate::wallet::{PrivateWallet, PublicWallet};
use crate::node::network::{NetworkConnection, Packet};
use crate::block::{Transaction, Block, BlockChain};

use std::sync::{Arc, Mutex};
use std::path::PathBuf;

pub struct TransactionCommand
{
    transaction_queue: Vec<Transaction>,
}

impl Default for TransactionCommand
{

    fn default() -> Self
    {
        Self
        {
            transaction_queue: Vec::new(),
        }
    }

}

impl Command for TransactionCommand
{

    fn name(&self) -> &'static str 
    { 
        "transaction"
    }

    fn invoke(&mut self, args: &[String], connection: &mut Arc<Mutex<NetworkConnection>>, chain: &mut BlockChain)
    {
        let from = PrivateWallet::read_from_file(&PathBuf::from(&args[0])).unwrap();
        let to = PrivateWallet::read_from_file(&PathBuf::from(&args[1])).unwrap();
        let amount = args[2].parse::<f64>().unwrap();
        let fee = args[2].parse::<f64>().unwrap();

        let transaction = Transaction::for_block(chain.longest_branch(), &from, &to, amount, fee).unwrap();
        NetworkConnection::broadcast(connection, None, 
            Packet::TransactionRequest(transaction.clone()));
        
        self.transaction_queue.push(transaction);
    }

    fn on_packet(&mut self, packet: Packet, connection: &mut Arc<Mutex<NetworkConnection>>, chain: &mut BlockChain)
    {
        match packet
        {
            Packet::TransactionRequest(transaction) =>
            {
                println!("Got tranaction {}", transaction.to_string());
                let wallet = PublicWallet::from_public_key_e(transaction.header.from, transaction.e);
                let balance = chain.longest_branch().lockup_wallet_status(&wallet).balance;
        
                if balance < transaction.header.amount + transaction.header.transaction_fee 
                    || !wallet.varify(&transaction.header.hash().unwrap(), &transaction.signature)
                {
                    println!("{} < {}", balance, transaction.header.amount + transaction.header.transaction_fee);
                    NetworkConnection::broadcast(connection, None, Packet::TransactionRequestRejected(transaction));
                    return;
                }
        
                self.transaction_queue.push(transaction.clone());
                NetworkConnection::broadcast(connection, None, Packet::TransactionRequestAccepted(transaction));
            },

            Packet::TransactionRequestAccepted(transaction) => println!("Accepted {}", transaction.to_string()),
            Packet::TransactionRequestRejected(transaction) => println!("Rejected {}", transaction.to_string()),
            _ => {},
        }
    }

    fn on_create_block(&mut self, block: &mut Block) 
    {
        // FIXME: If this fills up the block, we have a big problem
        for transaction in &self.transaction_queue 
        {
            println!("Adding {} to block {}", transaction.to_string(), block.block_id);
            block.add_transaction(transaction.clone());
        }
    }

    fn on_accepted_block(&mut self, block: &Block) 
    {
        for transaction in &block.transactions
        {
            let index = self.transaction_queue.iter().position(|x| x == transaction);
            if index.is_some() 
            {
                println!("Got {} in block {}", transaction.to_string(), block.block_id);
                self.transaction_queue.remove(index.unwrap());
            }
        }
    }

}
