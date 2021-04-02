extern crate sha2;
extern crate byteorder;

mod page;
mod transaction;
mod chain;
pub use page::Page;
pub use transaction::Transaction;
pub use chain::BlockChain;

use sha2::{Sha256, Digest};
use byteorder::{LittleEndian, ByteOrder};
use rsa::{RSAPublicKey, PublicKey, PaddingScheme, BigUint};
use slice_as_array;

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;
type Signature = [u8; PUB_KEY_LEN];
type Hash = [u8; HASH_LEN];

const BLOCK_HEADER_SIZE: usize = 
    HASH_LEN + // prev_hash
    8 + // block_id
    2 + // raward
    1 + // page count
    1 +  // transaction_count
    2 +  // difficulty 
    2 +  // pow 
    0;
const BLOCK_DATA_SIZE: usize = 16 * 1024 * 1024; // 16 MB
const BLOCK_SIZE: usize = BLOCK_HEADER_SIZE + BLOCK_DATA_SIZE;

#[derive(Debug, Clone)]
pub struct Block
{
    pub prev_hash: Hash,
    pub block_id: u64,
    pub raward: u16,
    pub raward_to: Signature,

    pub pages: Vec<Page>,
    pub transactions: Vec<Transaction>,
    pub difficulty: u16, // TODO: This should be a correct size
    pub pow: u16, // TODO: This should be a correct size
}

fn append_u64(vec: &mut Vec<u8>, n: u64)
{
    let mut buffer = [0u8; 8];
    LittleEndian::write_u64(&mut buffer, n);
    vec.extend_from_slice(&buffer);
}

fn append_u32(vec: &mut Vec<u8>, n: u32)
{
    let mut buffer = [0u8; 4];
    LittleEndian::write_u32(&mut buffer, n);
    vec.extend_from_slice(&buffer);
}

fn append_u16(vec: &mut Vec<u8>, n: u16)
{
    let mut buffer = [0u8; 2];
    LittleEndian::write_u16(&mut buffer, n);
    vec.extend_from_slice(&buffer);
}

impl Block
{

    pub fn new(prev: Option<&Block>, raward_to: Signature) -> Option<Self>
    {
        let prev_block_id = if prev.is_some() { prev.unwrap().block_id } else { 0 };
        let prev_block_hash = if prev.is_some() { prev.unwrap().hash()? } else { [0u8; HASH_LEN] };

        Some(Block
        {
            prev_hash: prev_block_hash,
            block_id: prev_block_id + 1,
            raward: 10, // TODO: This need to be done correctly
            raward_to: raward_to,

            pages: Vec::new(),
            transactions: Vec::new(),
            difficulty: 10,
            pow: 0,
        })
    }

    pub fn from_chain(chain: &BlockChain, raward_to: Signature) -> Option<Self>
    {
        let top = chain.top();
        if top.is_none() {
            Self::new(None, raward_to)
        } else {
            Self::new(Some( top.as_ref().unwrap() ), raward_to)
        }
    }

    pub fn add_page(&mut self, page: Page)
    {
        self.pages.push(page);
    }

    pub fn add_transaction(&mut self, transaction: Transaction)
    {
        self.transactions.push(transaction);
    }

    fn pages_as_bytes(&self) -> Option<Vec<u8>>
    {
        let mut bytes = Vec::<u8>::new();
        for page in &self.pages
        {
            bytes.push(page.page_data.len() as u8);
            bytes.extend_from_slice(&page.page_data);
            bytes.extend_from_slice(&page.site_id);
            bytes.extend_from_slice(&page.page_name);
            append_u32(&mut bytes, page.page_fee);
            bytes.extend_from_slice(&page.signature);
        }

        if bytes.len() > BLOCK_DATA_SIZE / 2 {
            None
        } else {
            Some( bytes )
        }
    }

    fn transactions_as_bytes(&self) -> Option<Vec<u8>>
    {
        let mut bytes = Vec::<u8>::new();
        for transaction in &self.transactions
        {
            append_u32(&mut bytes, transaction.id);
            bytes.extend_from_slice(&transaction.from);
            bytes.extend_from_slice(&transaction.to);
            append_u32(&mut bytes, transaction.amount);
            append_u32(&mut bytes, transaction.transaction_fee);
            bytes.extend_from_slice(&transaction.signature);
            bytes.extend_from_slice(&transaction.e);
        }

        if bytes.len() > BLOCK_DATA_SIZE / 2 {
            None
        } else {
            Some( bytes )
        }
    }

    pub fn as_bytes(&self) -> Option<Vec<u8>>
    {
        let mut bytes = Vec::<u8>::new();
        bytes.extend_from_slice(&self.prev_hash);
        append_u64(&mut bytes, self.block_id);
        append_u16(&mut bytes, self.raward);
        bytes.extend_from_slice(&self.raward_to);
        bytes.push(self.pages.len() as u8);
        bytes.push(self.transactions.len() as u8);
        append_u16(&mut bytes, self.difficulty);
        append_u16(&mut bytes, self.pow);

        let page_bytes = self.pages_as_bytes()?;
        let transaction_bytes = self.transactions_as_bytes()?;
        bytes.extend_from_slice(&page_bytes);
        bytes.extend_from_slice(&transaction_bytes);
        if bytes.len() > BLOCK_SIZE {
            None
        } else {
            Some( bytes )
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Self
    {
        // FIXME: This really isn't a good way of doing this
        let mut bp: usize = 0;
        let prev_hash = slice_as_array!(&bytes[bp..bp+HASH_LEN], [u8; HASH_LEN]).unwrap(); bp += HASH_LEN;
        let block_id = LittleEndian::read_u64(&bytes[bp..bp+8]); bp += 8;
        let raward = LittleEndian::read_u16(&bytes[bp..bp+2]); bp += 2;
        let raward_to = slice_as_array!(&bytes[bp..bp+PUB_KEY_LEN], [u8; PUB_KEY_LEN]).unwrap(); bp += PUB_KEY_LEN;
        let page_count = bytes[bp]; bp += 1;
        let transaction_count = bytes[bp]; bp += 1;
        let difficulty = LittleEndian::read_u16(&bytes[bp..bp+2]); bp += 2;
        let pow = LittleEndian::read_u16(&bytes[bp..bp+2]); bp += 2;

        let mut pages = Vec::<Page>::new();
        for _ in 0..page_count 
        {
            let data_len = bytes[bp] as usize; bp += 1;
            let data = bytes[bp..bp+data_len].to_vec(); bp += data_len;
            let site_id = slice_as_array!(&bytes[bp..bp+PUB_KEY_LEN], [u8; PUB_KEY_LEN]).unwrap(); bp += PUB_KEY_LEN;
            let page_name = slice_as_array!(&bytes[bp..bp+64], [u8; 64]).unwrap(); bp += 64;
            let fee = LittleEndian::read_u32(&bytes[bp..bp+4]); bp += 4;
            let signature = slice_as_array!(&bytes[bp..bp+PUB_KEY_LEN], [u8; PUB_KEY_LEN]).unwrap(); bp += PUB_KEY_LEN;
            pages.push(Page::new(data, *site_id, *page_name, fee, *signature));
        }

        let mut transactions = Vec::<Transaction>::new();
        for _ in 0..transaction_count 
        {
            let id = LittleEndian::read_u32(&bytes[bp..bp+4]); bp += 4;
            let from = slice_as_array!(&bytes[bp..bp+PUB_KEY_LEN], [u8; PUB_KEY_LEN]).unwrap(); bp += PUB_KEY_LEN;
            let to = slice_as_array!(&bytes[bp..bp+PUB_KEY_LEN], [u8; PUB_KEY_LEN]).unwrap(); bp += PUB_KEY_LEN;
            let amount = LittleEndian::read_u32(&bytes[bp..bp+4]); bp += 4;
            let fee = LittleEndian::read_u32(&bytes[bp..bp+4]); bp += 4;
            let signature = slice_as_array!(&bytes[bp..bp+PUB_KEY_LEN], [u8; PUB_KEY_LEN]).unwrap(); bp += PUB_KEY_LEN;
            let e = slice_as_array!(&bytes[bp..bp+3], [u8; 3]).unwrap(); bp += 3;
            transactions.push(Transaction::new(id, *from, *to, amount, fee, *signature, *e));
        }

        return Self
        {
            prev_hash: *prev_hash,
            block_id,
            raward,
            raward_to: *raward_to,

            pages,
            transactions,
            difficulty,
            pow,
        };
    }

    pub fn hash(&self) -> Option<Hash>
    {
        let bytes = self.as_bytes()?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);

        let hash = hasher.finalize();
        return Some( *slice_as_array!(&hash[0..HASH_LEN], [u8; HASH_LEN]).unwrap() );
    }

    /*
    fn print_as_bits(arr: &[u8])
    {
        for byte in arr
        {
            for i in 0..8 {
                print!("{}", (byte >> i) & 0x1);
            }
        }
        println!();
    }
    */

    fn validate_transactions(&self, _chain: &BlockChain) -> bool
    {
        for transaction in &self.transactions
        {
            let from = RSAPublicKey::new(BigUint::from_bytes_le(&transaction.from), BigUint::from_bytes_le(&transaction.e)).unwrap();
            let header = transaction.header_hash();
            if from.verify(PaddingScheme::new_pkcs1v15_sign(None), &header, &transaction.signature).is_err() {
                return false;
            }
        }

        // TODO: Varify balance
        // chain.lookup(&mut |block: &Block|
        // {
        // });

        return true;
    }

    pub fn validate(&self, chain: &BlockChain) -> bool
    {
        if self.block_id > 1
        {
            let last_block_or_none = chain.block(self.block_id - 1);
            if last_block_or_none.is_none() 
            {
                println!("prev is none");
                return false;
            }

            let last_block = last_block_or_none.unwrap();
            if self.block_id != last_block.block_id + 1 
            {
                println!("prev is not the last block");
                return false;
            }

            let prev_hash = last_block.hash();
            if prev_hash.is_none() 
            {
                println!("prev faild to hash");
                return false;
            }

            if self.prev_hash != prev_hash.unwrap() 
            {
                println!("prev hash does not match this hash");
                return false;
            }
        }

        let hash_or_none = self.hash();
        if hash_or_none.is_none() 
        {
            println!("faild to hash");
            return false;
        }

        // Validate POW
        let hash = hash_or_none.unwrap();
        for i in 0..(self.difficulty / 8 + 1) as usize
        {
            if i < (self.difficulty as usize / 8) && hash[i] != 0 {
                return false;
            }

            let zero_count = self.difficulty % 8;
            let mask = (1u8 << zero_count) - 1;
            if hash[i] & mask != 0 {
                return false;
            }
        }

        return self.validate_transactions(chain);
    }

}
