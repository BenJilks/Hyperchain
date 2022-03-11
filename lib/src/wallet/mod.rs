pub mod private_wallet;
pub mod public_wallet;
use crate::chain::BlockChain;
use crate::hash::{Hash, Signature};
use sha2::{Sha256, Digest};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WalletStatus
{
    pub balance: f32,
    pub max_id: u32,
}

impl Default for WalletStatus
{

    fn default() -> WalletStatus
    {
        WalletStatus
        {
            balance: 0.0,
            max_id: 0,
        }
    }

}

pub trait Wallet
{

    fn get_public_key(&self) -> Signature;

    fn get_address(&self) -> Hash
    {
        let mut hasher = Sha256::default();
        hasher.update(&self.get_public_key());
        Hash::from(&hasher.finalize())
    }

    fn get_status(&self, chain: &mut BlockChain) -> WalletStatus
        where Self: Sized
    {
        chain.get_wallet_status(&self.get_address())
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::private_wallet::PrivateWallet;
    use crate::block::Block;
    use crate::block::builder::BlockBuilder;
    use crate::transaction::builder::TransactionBuilder;
    use crate::transaction::transfer::TransferBuilder;
    use crate::chain::BlockChain;
    use crate::miner;

    #[test]
    fn test_wallet()
    {
        let mut chain = BlockChain::open_temp();
        let wallet = PrivateWallet::open_temp(0).unwrap();
        let other = PrivateWallet::open_temp(1).unwrap();

        let block_a = miner::mine_block(Block::new_blank(&mut chain, &wallet).expect("Create block"));
        chain.add(&block_a).unwrap();

        let block_b = miner::mine_block(Block::new_blank(&mut chain, &other).expect("Create block"));
        chain.add(&block_b).unwrap();
        
        let block_c = miner::mine_block(BlockBuilder::new(&wallet)
            .add_transfer(
                TransactionBuilder::new(
                    TransferBuilder::new(1, 0.2)
                        .add_output(other.get_address(), 4.6)
                        .build())
                    .add_input(&wallet, 4.6 + 0.2)
                    .build().unwrap())
            .add_transfer(
                TransactionBuilder::new(
                    TransferBuilder::new(1, 0.2)
                        .add_output(wallet.get_address(), 1.4)
                        .build())
                    .add_input(&other, 1.4 + 0.2)
                    .build().unwrap())
            .build(&mut chain)
            .expect("Create block"));
        chain.add(&block_c).unwrap();

        let wallet_status = wallet.get_status(&mut chain);
        assert_eq!(wallet_status.balance, block_a.calculate_reward() + block_c.calculate_reward() - 4.6 - 0.2 + 1.4 + 0.2 + 0.2);
        assert_eq!(wallet_status.max_id, 1);

        let other_status = other.get_status(&mut chain);
        assert_eq!(other_status.balance, block_b.calculate_reward() + (4.6 - 1.4 - 0.2));
        assert_eq!(other_status.max_id, 1);
    }

}

