use super::BlockChain;
use crate::wallet::WalletStatus;
use crate::block::{Block, Hash};

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct BlockMetadata
{
    pub wallets: HashMap<Hash, WalletStatus>,
}

impl BlockChain
{

    pub fn metadata_for_block(&mut self, block: &Block) -> BlockMetadata
    {
        // NOTE: We assume the block is valid at this point

        let mut wallets = HashMap::<Hash, WalletStatus>::new();
        for address in block.get_addresses_used() 
        {
            let mut status = self.get_wallet_status(&address);
            status = block.update_wallet_status(&address, status).unwrap();
            wallets.insert(address, status);
        }

        BlockMetadata
        {
            wallets: wallets,
        }
    }

}
