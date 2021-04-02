use super::{Signature, BlockChain, PUB_KEY_LEN};
use crate::wallet::{PrivateWallet, Wallet};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};
use bincode;

big_array! { BigArray; }

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct TransactionHeader
{
    pub id: u32,
    
    #[serde(with = "BigArray")]
    pub from: Signature,
    
    #[serde(with = "BigArray")]
    pub to: Signature,
    
    pub amount: u32,
    pub transaction_fee: u32,
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

    pub fn for_block(chain: &BlockChain, from: &PrivateWallet, to: Signature, amount: u32, fee: u32) -> Option<Self>
    {
        let status = from.calculate_status(chain);
        if amount + fee > status.balance {
            return None; // FIXME: Report invalid transaction error
        }

        let header = TransactionHeader 
        { 
            id: status.max_id + 1,
            from: from.get_public_key(),
            to,
            amount,
            transaction_fee: fee,
        };

        let signature_vec = from.sign(&header.hash().unwrap()).unwrap();
        let signature = *slice_as_array!(&signature_vec, [u8; PUB_KEY_LEN]).unwrap();
        Some( Self::new(header, signature, from.get_e()) )
    }

}
