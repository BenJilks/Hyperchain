use crate::block::{Hash, Block, PageHeader, DataFormat};
use crate::wallet::{PublicWallet, Wallet, WalletStatus};

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use std::path::PathBuf;
use std::fs::File;
use serde::{Serialize, Deserialize};

pub const CHUNK_SIZE: u64 = 100;

#[derive(Serialize, Deserialize, Debug)]
pub enum CumulativeDiff
{
    New(Vec<u8>),
    Diffs(Vec<Vec<u8>>),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockChainChunk
{
    bottom_id: u64,
    blocks: Vec<Option<Block>>,
    top: u64,

    ledger: HashMap<Hash, WalletStatus>,
    cumulative_page_diffs: HashMap<(Hash, String), CumulativeDiff>,
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
            cumulative_page_diffs: HashMap::new(),
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

    fn register_page_diff(&mut self, address: Hash, page: &PageHeader)
    {
        match self.cumulative_page_diffs.get_mut(&(address, page.page_name.clone()))
        {
            Some(CumulativeDiff::New(data)) =>
            {
                let patch = Cursor::new(page.page_data.clone());
                let older = Cursor::new(data.clone());
                let mut out_reader = bipatch::Reader::new(patch, older).unwrap();
        
                data.clear();
                out_reader.read_to_end(data).unwrap();
            },

            Some(CumulativeDiff::Diffs(diffs)) =>
            {
                diffs.push(page.page_data.clone());
            },

            None => 
            {
                self.cumulative_page_diffs.insert((address, page.page_name.clone()), 
                    CumulativeDiff::Diffs(vec![page.page_data.clone()]));
            },
        }
    }

    fn register_page_data(&mut self, page: &PageHeader)
    {
        let address = PublicWallet::from_public_key(page.site_id).get_address();
        match DataFormat::from_u8(page.data_format)
        {
            Some(DataFormat::NewRaw) => 
            {
                self.cumulative_page_diffs.insert((address, page.page_name.clone()), 
                    CumulativeDiff::New(page.page_data.clone()));
            },

            Some(DataFormat::DiffRaw) => 
            {
                self.register_page_diff(address, page);
            },
            
            None => panic!(),
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
            self.register_page_data(&page.header);
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

    pub fn apply_cumulative_page_diffs(&self, sites_path: &PathBuf)
    {
        for ((site_id, page_name), cumulative_diffs) in &self.cumulative_page_diffs
        {
            let page_path = sites_path
                .join(base_62::encode(site_id))
                .join(&page_name);
            
            std::fs::create_dir_all(page_path.parent().unwrap()).unwrap();
            match cumulative_diffs
            {
                CumulativeDiff::New(data) =>
                {
                    let mut file = File::create(&page_path).unwrap();
                    file.write(&data).unwrap();
                },

                CumulativeDiff::Diffs(diffs) =>
                {
                    for diff in diffs
                    {
                        let mut out = bipatch::Reader::new(&diff[..], File::open(&page_path).unwrap()).unwrap();
                        let mut buffer = Vec::<u8>::new();
                        out.read_to_end(&mut buffer).unwrap();
                        File::create(&page_path).unwrap().write(&buffer).unwrap();
                    }
                }
            }
        }
    }

}
