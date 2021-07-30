use super::{Signature, Hash, Branch, HASH_LEN, PUB_KEY_LEN};
use crate::wallet::{PrivateWallet, PublicWallet, Wallet, WalletStatus};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use bincode;

use std::string::ToString;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TransactionHeader
{
    pub id: u32,
    
    #[serde(with = "BigArray")]
    pub from: Signature,
    
    pub to: Hash,
    pub amount: f32,
    pub transaction_fee: f32,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct Transaction
{
    pub header: TransactionHeader,

    #[serde(with = "BigArray")]
    pub signature: Signature,
    
    pub e: [u8; 3],
}

impl TransactionHeader
{
     
    pub fn hash(&self) -> Option<Vec<u8>>
    {
        let result = bincode::serialize(self);
        if result.is_err() {
            return None;
        }

        let mut hasher = Sha256::new();
        hasher.update(&result.unwrap());
        Some( hasher.finalize().to_vec() )
    }

}

impl Transaction
{

    pub fn new(header: TransactionHeader, signature: Signature, e: [u8; 3]) -> Self
    {
        Self
        {
            header,
            signature,
            e,
        }
    }

    pub fn for_chain<W: Wallet>(chain: Option<&Branch>, from: &PrivateWallet, to: &W, amount: f32, fee: f32) -> Option<Self>
    {
        let status = 
            if chain.is_some() { 
                from.get_status(chain.unwrap())
            } else {
                WalletStatus::default()
            };

        let header = TransactionHeader 
        { 
            // TODO: This id should be calculated correctly
            id: status.max_id + 1,
            from: from.get_public_key(),
            to: to.get_address(),
            amount,
            transaction_fee: fee,
        };

        let signature_vec = from.sign(&header.hash().unwrap()).unwrap();
        let signature = *slice_as_array!(&signature_vec, [u8; PUB_KEY_LEN]).unwrap();
        Some( Self::new(header, signature, from.get_e()) )
    }

    pub fn is_valid(&self) -> bool
    {
        if self.header.amount < 0.0 {
            return false;
        }

        if self.header.transaction_fee < 0.0 {
            return false;
        }

        let wallet = PublicWallet::from_public_key_e(self.header.from, self.e);
        let header = self.header.hash().expect("Hash header");
        wallet.verify(&header, &self.signature)
    }

    pub fn get_from_address(&self) -> [u8; HASH_LEN]
    {
        let mut hasher = Sha256::new();
        hasher.update(&self.header.from);

        let hash = hasher.finalize().to_vec();
        *slice_as_array!(&hash, [u8; HASH_LEN]).unwrap()
    }

}

impl ToString for Transaction
{

    fn to_string(&self) -> String
    {
        format!("{}... --[ {} + {}tx ]--> {}...", 
            &base_62::encode(&self.header.from)[0..10],
            self.header.amount,
            self.header.transaction_fee,
            &base_62::encode(&self.header.to)[0..10])
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use crate::block::{Block, BlockChain};
    use crate::logger::{Logger, LoggerLevel};
    use crate::miner;
    use std::path::PathBuf;

    #[test]
    fn test_transaction()
    {
        let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Error);
        let mut chain = BlockChain::new(&mut logger);
        let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();

        let block = miner::mine_block(Block::new(chain.current_branch(), &wallet).expect("Create block"));
        chain.add(&block, &mut logger);

        {
            let transaction = Transaction::for_chain(chain.current_branch(), &wallet, &other, 2.4, 0.2)
                .expect("Create transaction");
            transaction.header.hash().expect("Hash header");
            assert_eq!(transaction.is_valid(), true);
            assert_eq!(transaction.to_string(), "aLOExVDb0w... --[ 2.4 + 0.2tx ]--> zCPOqvKFuo...");
        }

        {
            let transaction = Transaction::for_chain(chain.current_branch(), &wallet, &other, -1.6, 0.0)
                .expect("Create transaction");
            assert_eq!(transaction.is_valid(), false);
        }

        {
            let transaction = Transaction::for_chain(chain.current_branch(), &wallet, &other, 0.0, -0.0001)
                .expect("Create transaction");
            assert_eq!(transaction.is_valid(), false);
        }
    }

}

