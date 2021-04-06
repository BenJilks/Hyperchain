use crate::block::{Hash, Block};
use crate::wallet::{PublicWallet, Wallet, WalletStatus};

use std::collections::HashMap;
use serde::{Serialize, Deserialize};

pub const CHUNK_SIZE: u64 = 100;

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockChainChunk
{
    bottom_id: u64,
    blocks: Vec<Option<Block>>,
    top: u64,

    ledger: HashMap<Hash, WalletStatus>,
}

impl BlockChainChunk
{

    pub fn new(chunk_id: u64) -> Self
    {
        Self
        {
            bottom_id: chunk_id * CHUNK_SIZE,
            blocks: vec![None; CHUNK_SIZE as usize],
            top: 0u64,
            ledger: HashMap::new(),
        }
    }

    fn change_ledger(&mut self, address: &Hash, change: f64, id: u32)
    {
        match self.ledger.get_mut(address)
        {
            Some(entry) => 
            {
                entry.balance += change;
                entry.max_id = std::cmp::max(entry.max_id, id);
            },

            None => 
            { 
                self.ledger.insert(address.clone(), WalletStatus 
                {  
                    balance: change,
                    max_id: id,
                }); 
            },
        }
    }

    pub fn add(&mut self, block: &Block)
    {
        self.change_ledger(&block.raward_to, block.calculate_reward(), 0);

        for transaction in &block.transactions 
        {
            let from_address = PublicWallet::from_public_key(transaction.header.from).get_address();
            self.change_ledger(&from_address, -transaction.header.amount - transaction.header.transaction_fee, transaction.header.id);
            self.change_ledger(&transaction.header.to, transaction.header.amount, transaction.header.id);
            self.change_ledger(&block.raward_to, transaction.header.transaction_fee, 0);
        }

        for page in &block.pages
        {
            let owner_address = PublicWallet::from_public_key(page.header.site_id).get_address();
            self.change_ledger(&owner_address, -page.header.page_fee, 0);
            self.change_ledger(&block.raward_to, page.header.page_fee, 0);
        }

        self.blocks[(block.block_id - self.bottom_id) as usize] = Some( block.clone() );
        self.top = std::cmp::max(self.top, block.block_id - self.bottom_id);
    }

    pub fn block(&self, id: u64) -> Option<Block>
    {
        if id < self.bottom_id || id > self.bottom_id + self.top {
            return None
        } else {
            self.blocks[(id - self.bottom_id) as usize].clone()
        }
    }

    pub fn top_index(&self) -> u64
    {
        self.bottom_id + self.top
    }

    pub fn wallet_status_change<W: Wallet>(&self, wallet: &W) -> WalletStatus
    {
        match self.ledger.get(&wallet.get_address())
        {
            Some(change) => change.clone(),

            None => WalletStatus 
            {  
                balance: 0f64,
                max_id: 0,
            },
        }
    }

}
