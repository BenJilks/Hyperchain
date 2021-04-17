use super::{Chunk, CHUNK_SIZE};
use crate::block::{Hash, Block, PageHeader, DataFormat};
use crate::wallet::{PublicWallet, Wallet, WalletStatus};
use crate::error::Error;

use std::path::PathBuf;
use std::fs::File;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CumulativeDiff
{
    New(Vec<u8>),
    Diffs(Vec<Vec<u8>>),
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MainChunk
{
    pub chunk_id: u64,
    local_top: usize,
    blocks: Vec<Option<Block>>,

    ledger: HashMap<Hash, WalletStatus>,
    cumulative_page_diffs: HashMap<(Hash, String), CumulativeDiff>,
}

impl MainChunk
{

    fn new(path: PathBuf, chunk_id: u64) -> Self
    {
        std::fs::create_dir_all(path).unwrap();

        Self
        {
            chunk_id: chunk_id,
            local_top: 0,
            blocks: vec![None; CHUNK_SIZE as usize],

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

    fn block_id_to_local_id(block_id: u64) -> usize
    {
        (block_id % CHUNK_SIZE) as usize
    }

    fn accumulate_data(&mut self, block: &Block)
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

    pub fn top(&self) -> Option<Block>
    {
        self.blocks[self.local_top].clone()
    }

}

impl Chunk for MainChunk
{

    fn from(path: PathBuf, chunk_id: u64) -> Self
    {
        let chunk_path = path.join(chunk_id.to_string());
        if !chunk_path.exists() {
            return Self::new(path, chunk_id);
        }

        let chunk_file = File::open(chunk_path).unwrap();
        bincode::deserialize_from::<File, Self>(chunk_file).unwrap()
    }

    fn write(&self, path: PathBuf)
    {
        let chunk_path = path.join(self.chunk_id.to_string());
        let chunk_file = File::create(chunk_path).unwrap();
        bincode::serialize_into(chunk_file, self).unwrap();
    }

    fn block(&self, block_id: u64) -> Option<Block>
    {
        let local_id = Self::block_id_to_local_id(block_id);
        self.blocks[local_id].clone()
    }

    fn set_block(&mut self, block: Block) -> Result<(), Error>
    {
        let local_id = Self::block_id_to_local_id(block.block_id);
        if block.block_id > 1 && local_id != 0
        {
            if local_id != self.local_top + 1 {
                return Err(Error::NotNextBlock);
            }
        }

        self.accumulate_data(&block);
        self.blocks[local_id] = Some( block );
        self.local_top = local_id;
        Ok(())
    }

}

#[cfg(test)]
mod tests
{
    use super::*;
    use crate::block::{Transaction, TransactionHeader};
    use crate::wallet::PublicWallet;

    fn write_blocks_to_chunk()
    {
        let wallet = PublicWallet::from_public_key([0u8; 256]);
        let other = PublicWallet::from_public_key([1u8; 256]);

        let mut a: MainChunk = Chunk::from(PathBuf::from("main_chunk_tests_temp"), 0);
        assert_eq!(a.set_block(Block
        {
            prev_hash: [0u8; 32],
            block_id: 1,
            raward_to: wallet.get_address(),
            pages: Vec::new(),
            transactions: vec![Transaction
            {
                header: TransactionHeader
                {
                    id: 0,
                    from: wallet.get_public_key(),
                    to: other.get_address(),
                    amount: 1.0,
                    transaction_fee: 1.0,
                },
                signature: [0u8; 256],
                e: [0u8; 3],
            }],
            timestamp: 0,
            target: [0u8; 32],
            pow: 0,
        }).is_ok(), true);

        assert_eq!(a.set_block(Block
        {
            prev_hash: [0u8; 32],
            block_id: 2,
            raward_to: wallet.get_address(),
            pages: Vec::new(),
            transactions: vec![Transaction
            {
                header: TransactionHeader
                {
                    id: 1,
                    from: wallet.get_public_key(),
                    to: other.get_address(),
                    amount: 1.0,
                    transaction_fee: 1.0,
                },
                signature: [0u8; 256],
                e: [0u8; 3],
            }],
            timestamp: 0,
            target: [0u8; 32],
            pow: 0,
        }).is_ok(), true);

        assert_eq!(a.set_block(Block
        {
            prev_hash: [0u8; 32],
            block_id: 5,
            raward_to: wallet.get_address(),
            pages: Vec::new(),
            transactions: Vec::new(),
            timestamp: 0,
            target: [0u8; 32],
            pow: 0,
        }).is_ok(), false);
    
        a.write(PathBuf::from("main_chunk_tests_temp"));
    }

    fn test_chunk_has_blocks()
    {
        let a: MainChunk = Chunk::from(PathBuf::from("main_chunk_tests_temp"), 0);
        assert_eq!(a.blocks.iter().filter(|x| x.is_some()).count(), 2);

        let wallet = PublicWallet::from_public_key([0u8; 256]);
        let other = PublicWallet::from_public_key([1u8; 256]);

        let test_block = |block_or_none: &Option<Block>, id: u64|
        {
            assert_eq!(block_or_none.is_some(), true);

            let block = block_or_none.clone().unwrap();
            assert_eq!(block.prev_hash, [0u8; 32]);
            assert_eq!(block.block_id, id);
            assert_eq!(block.raward_to, wallet.get_address());
            assert_eq!(block.pages, Vec::new());
            assert_eq!(block.timestamp, 0);
            assert_eq!(block.target, [0u8; 32]);
            assert_eq!(block.pow, 0);
        };
        test_block(&a.blocks[1], 1);
        test_block(&a.blocks[2], 2);
        assert_eq!(a.top(), a.blocks[2]);

        assert_eq!(a.wallet_status_change(&wallet).balance, 18.0);
        assert_eq!(a.wallet_status_change(&other).balance, 2.0);
    }

    fn clean_up()
    {
        assert_eq!(std::fs::remove_dir_all(PathBuf::from("main_chunk_tests_temp")).is_ok(), true);
    }

    #[test]
    fn test_main_chunk()
    {
        write_blocks_to_chunk();
        test_chunk_has_blocks();
        clean_up();
    }

}
