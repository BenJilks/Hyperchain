use super::{PublicKey, Signature};

#[derive(Debug, Clone)]
pub struct Transaction
{
    pub from: PublicKey,
    pub to: PublicKey,
    pub transaction_fee: u32,
    pub signature: Signature,
}

impl Transaction
{

    pub fn new(from: PublicKey, to: PublicKey, fee: u32, signature: Signature) -> Self
    {
        Self
        {
            from,
            to,
            transaction_fee: fee,
            signature,
        }
    }

}
