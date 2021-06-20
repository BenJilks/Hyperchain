use super::{Signature, Hash, BlockChain, PUB_KEY_LEN};
use crate::wallet::{PrivateWallet, Wallet};
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
    pub amount: f64,
    pub transaction_fee: f64,
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

    pub fn for_block<W: Wallet>(chain: &BlockChain, from: &PrivateWallet, to: &W, amount: f64, fee: f64) -> Option<Self>
    {
        return None;

        /*
        let status = chain.lockup_wallet_status(from);
        if amount + fee > status.balance {
            return None; // FIXME: Report invalid transaction error
        }

        let header = TransactionHeader 
        { 
            id: status.max_id + 1,
            from: from.get_public_key(),
            to: to.get_address(),
            amount,
            transaction_fee: fee,
        };

        let signature_vec = from.sign(&header.hash().unwrap()).unwrap();
        let signature = *slice_as_array!(&signature_vec, [u8; PUB_KEY_LEN]).unwrap();
        Some( Self::new(header, signature, from.get_e()) )
        */
    }

}

impl ToString for Transaction
{

    fn to_string(&self) -> String
    {
        format!("{}... -- {} + {}tx --> {}...", 
            &base_62::encode(&self.header.from)[0..10],
            self.header.amount,
            self.header.transaction_fee,
            &base_62::encode(&self.header.to)[0..10])
    }

}
