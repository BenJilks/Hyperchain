use super::{Signature, BlockChain, Block, PUB_KEY_LEN};
use crate::wallet::Wallet;
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

    pub fn new(id: u32, from: Signature, to: Signature, amount: u32, fee: u32, signature: Signature, e: [u8; 3]) -> Self
    {
        Self
        {
            header: TransactionHeader
            {
                id,
                from,
                to,
                amount,
                transaction_fee: fee,
            },
            signature,
            e,
        }
    }

    pub fn from_header(header: TransactionHeader, signature: Signature, e: [u8; 3]) -> Self
    {
        Self
        {
            header,
            signature,
            e,
        }
    }

    pub fn for_block(chain: &BlockChain, from: &Wallet, to: Signature, amount: u32, fee: u32) -> Option<Self>
    {
        // Find the next unique id
        let mut id = 0;
        let mut balance = 0u32;
        let from_pub_key = from.get_public_key();

        chain.lookup(&mut |block: &Block|
        {
            let mut is_miner = false;
            if block.raward_to == from_pub_key 
            {
                balance += block.raward as u32;
                is_miner = true;
            }

            for transaction in &block.transactions
            {
                if transaction.header.from == from_pub_key
                {
                    id = std::cmp::max(id, transaction.header.id);
                    balance -= transaction.header.amount;
                    balance -= transaction.header.transaction_fee;
                }

                if transaction.header.to == from_pub_key {
                    balance += transaction.header.amount;
                }

                if is_miner {
                    balance += transaction.header.transaction_fee;
                }
            }
        });

        if amount + fee > balance {
            return None; // FIXME: Report invalid transaction error
        }

        let header = TransactionHeader { id, from: from_pub_key, to, amount, transaction_fee: fee };
        let signature = from.sign(&header.hash().unwrap()).unwrap();
        Some( Self::from_header(header, *slice_as_array!(&signature, [u8; PUB_KEY_LEN]).unwrap(), from.get_e()) )
    }

}
