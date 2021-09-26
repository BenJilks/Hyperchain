use super::{Input, Transaction, TransactionHeader, TransactionContent};
use crate::wallet::Wallet;
use crate::wallet::private_wallet::PrivateWallet;

use serde::Serialize;
use std::collections::HashMap;
use std::error::Error;

pub struct TransactionBuilder<'a, C>
    where C: TransactionContent
{
    content: C,
    inputs: Vec<(&'a PrivateWallet, Input)>,
}

impl<'a, C> TransactionBuilder<'a, C>
    where C: TransactionContent + Serialize
{

    pub fn new(content: C) -> Self
    {
        Self
        {
            content,
            inputs: Vec::new(),
        }
    }

    pub fn add_input(mut self, wallet: &'a PrivateWallet, amount: f32) -> Self
    {
        let input = Input
        {
            from: wallet.get_public_key(),
            e: wallet.get_e(),
            amount,
        };
        
        self.inputs.push((wallet, input));
        self
    }

    pub fn build(self) -> Result<Transaction<C>, Box<dyn Error>>
    {
        let header = TransactionHeader
        {
            content: self.content,
            inputs: self.inputs.iter().map(|x| x.1.clone()).collect::<Vec<_>>(),
        };

        let header_hash = header.hash()?;
        let mut signatures = HashMap::new();
        for (wallet, input) in &self.inputs 
        {
            let signature = wallet.sign(&header_hash)?;
            signatures.insert(input.get_address(), signature);
        }

        Ok(Transaction::new(header, signatures))
    }

}
