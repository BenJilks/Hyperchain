use super::{Signature, BlockChain, Block, append_u32, PUB_KEY_LEN};
use crate::wallet::Wallet;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone)]
pub struct Transaction
{
    pub id: u32,
    pub from: Signature,
    pub to: Signature,
    pub amount: u32,
    pub transaction_fee: u32,
    pub signature: Signature,
    pub e: [u8; 3],
}

impl Transaction
{

    pub fn new(id: u32, from: Signature, to: Signature, amount: u32, fee: u32, signature: Signature, e: [u8; 3]) -> Self
    {
        Self
        {
            id,
            from,
            to,
            amount,
            transaction_fee: fee,
            signature,
            e,
        }
    }

    fn header_hash_impl(id: u32, to: &Signature, from: &Signature, amount: u32, fee: u32) -> Vec<u8>
    {
        let mut bytes = Vec::<u8>::new();
        append_u32(&mut bytes, id);
        bytes.extend_from_slice(to);
        bytes.extend_from_slice(from);
        append_u32(&mut bytes, amount);
        append_u32(&mut bytes, fee);

        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().to_vec()
    }

    pub fn header_hash(&self) -> Vec<u8>
    {
        Self::header_hash_impl(self.id, &self.to, &self.from, self.amount, self.transaction_fee)
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
                if transaction.from == from_pub_key
                {
                    id = std::cmp::max(id, transaction.id);
                    balance -= transaction.amount;
                    balance -= transaction.transaction_fee;
                }

                if transaction.to == from_pub_key {
                    balance += transaction.amount;
                }

                if is_miner {
                    balance += transaction.transaction_fee;
                }
            }
        });

        if amount + fee > balance {
            return None; // FIXME: Report invalid transaction error
        }

        let signature = from.sign(&Self::header_hash_impl(id, &from_pub_key, &to, amount, fee)).unwrap();
        Some( Self::new(id, from_pub_key, to, amount, fee, *slice_as_array!(&signature, [u8; PUB_KEY_LEN]).unwrap(), from.get_e()) )
    }

}
