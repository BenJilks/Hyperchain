use super::BlockChain;
use crate::transaction::{Transaction, TransactionContent, TransactionVariant};
use crate::transaction::page::Page;
use crate::block::Block;
use crate::wallet::WalletStatus;
use crate::config::Hash;

use serde::Serialize;

fn find_transaction<C>(transactions: &Vec<Transaction<C>>, transaction_id: &Hash)
        -> Option<Transaction<C>>
    where C: TransactionContent + Serialize + Clone
{
    for transaction in transactions
    {
        match transaction.hash()
        {
            Ok(hash) =>
            {
                if hash == transaction_id {
                    return Some(transaction.clone());
                }
            },
            Err(_) => {},
        }
    }

    None
}

impl BlockChain
{

    pub fn get_wallet_status_up_to_block(&mut self, to: u64, address: &Hash) -> WalletStatus
    {
        let real_to = std::cmp::min(to + 1, self.blocks.next_top());
        for block_id in (0..real_to).rev()
        {
            let metadata = self.metadata.get(block_id).unwrap();
            if metadata.wallets.contains_key(address) {
                return metadata.wallets[address].clone();
            }
        }

        WalletStatus::default()
    }

    pub fn get_wallet_status(&mut self, address: &Hash) -> WalletStatus
    {
        if self.blocks.next_top() == 0 {
            WalletStatus::default()
        } else {
            self.get_wallet_status_up_to_block(self.blocks.next_top() - 1, address)
        }
    }

    pub fn last_page_update(&mut self, address: &Hash) -> Option<Block>
    {
        for block_id in (0..self.blocks.next_top()).rev()
        {
            let metadata = self.metadata.get(block_id).unwrap();
            if metadata.page_updates.contains_key(address) {
                return self.blocks.get(block_id);
            }
        }

        None
    }

    pub fn get_page_updates(&mut self, address: &Hash) 
        -> Vec<Transaction<Page>>
    {
        // FIXME: Extremely slow, need to use metadata to 
        //        optimise this!

        let mut updates = Vec::new();
        for block_id in (0..self.blocks.next_top()).rev()
        {
            let metadata = self.metadata.get(block_id).unwrap();
            if !metadata.page_updates.contains_key(address) {
                continue;
            }

            let block = self.block(block_id).unwrap();
            for page in block.pages.iter().rev() 
            {
                if page.get_from_addresses().contains(address) {
                    updates.push(page.clone());
                }
            }

            if metadata.page_updates[address].is_creation {
                break;
            }
        }

        updates.reverse();
        updates
    }

    pub fn find_transaction_in_chain(&mut self, transaction_id: &Hash) 
        -> Option<(TransactionVariant, Block)>
    {
        // FIXME: Extremely slow, need to use metadata to 
        //        optimise this!

        for block_id in 0..self.blocks.next_top() 
        {
            let block = self.block(block_id).unwrap();

            let transfer = find_transaction(&block.transfers, transaction_id);
            if transfer.is_some() {
                return Some((TransactionVariant::Transfer(transfer.unwrap()), block.clone()));
            }

            let page = find_transaction(&block.pages, transaction_id);
            if page.is_some() {
                return Some((TransactionVariant::Page(page.unwrap()), block.clone()));
            }
        }

        None
    }

    pub fn get_transaction_history(&mut self, address: &Hash) 
        -> Vec<(TransactionVariant, Option<Block>)>
    {
        // FIXME: Extremely slow, need to use metadata to 
        //        optimise this!

        let mut transactions = Vec::<(TransactionVariant, Option<Block>)>::new();
        self.walk(&mut |block|
        {
            for transfer in &block.transfers 
            {
                if transfer.header.content.outputs.iter().any(|x| &x.to == address) ||
                    transfer.get_from_addresses().contains(&address)
                {
                    transactions.push((
                        TransactionVariant::Transfer(transfer.clone()),
                        Some(block.clone())));
                }
            }

            for page in &block.pages
            {
                if page.get_from_addresses().contains(&address)
                {
                    transactions.push((
                        TransactionVariant::Page(page.clone()), 
                        Some(block.clone())));
                }
            }
        });

        for transfer in self.transfer_queue.transactions()
        {
            if transfer.header.content.outputs.iter().any(|x| &x.to == address) ||
                transfer.get_from_addresses().contains(&address)
            {
                transactions.push((
                    TransactionVariant::Transfer(transfer.clone()), 
                    None));
            }
        }

        for page in self.page_queue.transactions()
        {
            if page.get_from_addresses().contains(&address)
            {
                transactions.push((
                    TransactionVariant::Page(page.clone()), 
                    None));
            }
        }

        transactions.reverse();
        transactions
    }

}

#[cfg(test)]
mod tests
{

    use super::*;
    use super::super::BlockChainAddResult;
    use crate::block::builder::BlockBuilder;
    use crate::wallet::Wallet;
    use crate::wallet::private_wallet::PrivateWallet;
    use crate::data_store::DataUnit;
    use crate::data_store::page::CreatePageData;
    use crate::miner;
    use crate::config::HASH_LEN;

    #[test]
    fn test_chain_transaction()
    {
        let _ = pretty_env_logger::try_init();

        let mut chain = BlockChain::open_temp();
        let wallet = PrivateWallet::open_temp(0).unwrap();
        let other = PrivateWallet::open_temp(1).unwrap();

        let block_a = miner::mine_block(Block::new_blank(&mut chain, &wallet).unwrap());
        assert_eq!(chain.add(&block_a).unwrap(), BlockChainAddResult::Ok);

        // Create transfer
        let transaction = chain.new_transfer(vec![(&wallet, 2.0)], vec![(other.get_address(), 2.0)], 0.0).unwrap().unwrap();
        assert_eq!(chain.push_transfer_queue(transaction.clone()).unwrap(), true);

        // Create page
        let page_data = CreatePageData::new("index.html".to_owned(), Vec::new());
        let page = chain.new_page(&wallet, &DataUnit::CreatePage(page_data), 0.0).unwrap().unwrap();
        assert_eq!(chain.push_page_queue(page.clone()).unwrap(), true);

        // Add transactions to new block
        let block_b = miner::mine_block(BlockBuilder::new(&wallet)
            .add_transfer(transaction.clone())
            .add_page(page.clone())
            .build(&mut chain)
            .unwrap());
        assert_eq!(chain.add(&block_b).unwrap(), BlockChainAddResult::Ok);

        // Test 'get_page_updates'
        assert_eq!(chain.get_page_updates(&wallet.get_address()), 
                   [page.clone()]);

        // Test 'find_transaction_in_chain'
        let transaction_id_vec = transaction.hash().unwrap();
        let transaction_id = slice_as_array!(&transaction_id_vec, [u8; HASH_LEN]).unwrap();
        assert_eq!(chain.find_transaction_in_chain(transaction_id), 
                   Some((TransactionVariant::Transfer(transaction.clone()), block_b.clone())));

        // Test 'push_transfer_queue'
        let other_transaction = chain.new_transfer(vec![(&wallet, 2.0)], vec![(other.get_address(), 2.0)], 0.0).unwrap().unwrap();
        assert_eq!(chain.push_transfer_queue(other_transaction.clone()).unwrap(), true);

        // Test 'get_transaction_history'
        assert_eq!(chain.get_transaction_history(&wallet.get_address()), 
                   [
                       (TransactionVariant::Transfer(other_transaction), None),
                       (TransactionVariant::Page(page), Some(block_b.clone())),
                       (TransactionVariant::Transfer(transaction), Some(block_b.clone())),
                   ]);
    }

}

