use super::Block;
use crate::chain::BlockChain;
use crate::transaction::Transaction;
use crate::transaction::transfer::Transfer;
use crate::transaction::page::Page;
use crate::wallet::Wallet;

use std::error::Error;

pub struct BlockBuilder<'a, W>
    where W: Wallet
{
    raward_to: &'a W,
    transfers: Vec<Transaction<Transfer>>,
    pages: Vec<Transaction<Page>>,
}

impl<'a, W> BlockBuilder<'a, W>
    where W: Wallet
{

    pub fn new(raward_to: &'a W) -> Self
    {
        Self
        {
            raward_to,
            transfers: Vec::new(),
            pages: Vec::new(),
        }
    }

    pub fn add_transfer(mut self, transfer: Transaction<Transfer>) -> Self
    {
        self.transfers.push(transfer);
        self
    }

    pub fn add_page(mut self, page: Transaction<Page>) -> Self
    {
        self.pages.push(page);
        self
    }

    pub fn build(self, chain: &mut BlockChain) -> Result<Block, Box<dyn Error>>
    {
        Block::new(chain, self.raward_to, 
            self.transfers, self.pages)
    }

}
