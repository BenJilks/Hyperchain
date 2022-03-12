/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use super::BlockChain;
use crate::wallet::WalletStatus;
use crate::block::Block;
use crate::hash::Hash;

use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct PageMetadata
{
    pub is_creation: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct BlockMetadata
{
    pub wallets: HashMap<Hash, WalletStatus>,
    pub page_updates: HashMap<Hash, PageMetadata>,
}

impl BlockChain
{

    pub fn metadata_for_block(&mut self, block: &Block) -> BlockMetadata
    {
        // NOTE: We assume the block is valid at this point

        let mut wallets = HashMap::new();
        for address in block.get_addresses_used() 
        {
            let mut status = self.get_wallet_status(&address);
            status = block.update_wallet_status(&address, status).unwrap();
            wallets.insert(address, status);
        }

        let mut page_updates = HashMap::new();
        for page in &block.pages 
        {
            let is_creation = self.last_page_update(&page.header.content.site).is_none();
            page_updates.insert(page.header.content.site, PageMetadata
            {
                is_creation,
            });
        }

        BlockMetadata
        {
            wallets,
            page_updates,
        }
    }

}

