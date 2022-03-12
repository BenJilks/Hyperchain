/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

pub mod packet_handler;
use crate::network::packet::Packet;
use crate::network::client_manager::ClientManager;
use crate::report::{Report, NodeReport};

use libhyperchain::chain::{BlockChain, BlockChainAddResult};
use libhyperchain::chain::branch::BlockChainCanMergeResult;
use libhyperchain::block::Block;
use libhyperchain::data_store::DataStore;
use libhyperchain::data_store::data_unit::DataUnit;
use libhyperchain::transaction::Transaction;
use libhyperchain::transaction::transfer::Transfer;
use libhyperchain::transaction::page::Page;
use libhyperchain::hash::Hash;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::error::Error;

pub struct Node
{
    port: u16,
    chain: BlockChain,
    data_store: DataStore,
    report: Report,
    branches: HashMap<String, Vec<Block>>,
}

impl Node
{

    pub fn new(port: u16, path: &PathBuf) -> Result<Arc<Mutex<Self>>, Box<dyn Error>>
    {
        let chain = BlockChain::open(&path.join("blockchain"))?;
        let data_store = DataStore::open(&path.join("data"))?;
        let report = Report::open(&path.join("report.json"));

        Ok(Arc::from(Mutex::from(Self
        {
            port,
            chain,
            data_store,
            report,
            branches: HashMap::new()
        })))
    }

    pub fn chain(&mut self) -> &mut BlockChain
    {
        &mut self.chain
    }

    pub fn data_store(&mut self) -> &mut DataStore
    {
        &mut self.data_store
    }

    pub fn our_report(&self) -> Result<NodeReport, Box<dyn Error>>
    {
        Ok(NodeReport::new(self.data_store.report()?))
    }

    pub fn storage_usage(&self) -> Result<HashMap<Hash, usize>, Box<dyn Error>>
    {
        let mut usage = self.report.storage_usage();
        for chunk in self.data_store.report()?
        {
            if !usage.contains_key(&chunk) {
                usage.insert(chunk.clone(), 0);
            }
            *usage.get_mut(&chunk).unwrap() += 1;
        }

        Ok(usage)
    }

    fn try_insert_block_into_branch(branch: &mut Vec<Block>, block: Block) 
        -> bool
    {
        // Is start of new branch
        if branch.is_empty() 
        {
            branch.push(block);
            return true;
        }

        // Can be added to the bottom
        let bottom = branch.first().unwrap();
        if bottom.validate_next(&block).is_ok() 
        {
            branch.insert(0, block);
            return true;
        }

        // Can be added to the top
        let top = branch.last().unwrap();
        if block.validate_next(top).is_ok() {
            branch.push(block);
            return true;
        }

        // Already exists
        let bottom_id = bottom.header.block_id;
        let top_id = top.header.block_id;
        if (bottom_id..=top_id).contains(&block.header.block_id) 
        {
            let branch_index = (block.header.block_id - bottom_id) as usize;
            let existing_block_in_branch = branch.get(branch_index).unwrap();
            if &block == existing_block_in_branch {
                return true;
            }
        }

        false
    }

    fn add_to_branch(&mut self, from: &str, block: Block)
        -> Option<u64>
    {
        if !self.branches.contains_key(from) {
            self.branches.insert(from.to_owned(), Vec::new());
        }

        let mut branch = self.branches.remove(from).unwrap();
        if !Self::try_insert_block_into_branch(&mut branch, block) {
            return None;
        }
        
        let bottom = branch.first().unwrap();
        let bottom_id = bottom.header.block_id;
        self.branches.insert(from.to_owned(), branch);

        if bottom_id == 0 {
            None
        } else {
            Some(bottom_id - 1)
        }
    }

    fn complete_branch(&mut self, from: &str) -> Result<(), Box<dyn Error>>
    {
        if !self.branches.contains_key(from) {
            return Ok(());
        }

        let branch = self.branches.remove(from).unwrap();
        if self.chain.can_merge_branch(&branch)? == BlockChainCanMergeResult::Ok
        {
            info!("[{}] Merge longer branch", self.port);
            self.chain.merge_branch(branch);
        }
        Ok(())
    }

    fn should_ignore_block(&mut self, from: &str, block: &Block)
        -> bool
    {
        let top_or_none = self.chain.top();
        if top_or_none.is_none() {
            return false;
        }

        let top = top_or_none.unwrap();
        let top_id = top.header.block_id;
        match self.branches.get(from)
        {
            Some(branch) =>
            {
                let branch_top = branch.last().unwrap();
                branch_top.header.block_id < top_id
            },

            None => block.header.block_id < top_id,
        }
    }

    fn handle_block(&mut self, manager: &mut ClientManager, from: &str, 
                    block: Block) 
        -> Result<(), Box<dyn Error>>
    {
        if self.should_ignore_block(from, &block) {
            return Ok(());
        }

        match self.chain.add(&block)?
        {
            BlockChainAddResult::Ok =>
            {
                info!("[{}] Added block {}", self.port, block.header.block_id);

                // Relay this block to the rest of the network
                manager.send(Packet::Block(block.clone()))?;
            },

            BlockChainAddResult::Invalid(_) | BlockChainAddResult::MoreNeeded => 
            {
                info!("[{}] Invalid block {}", self.port, block.header.block_id);

                // Add block to this nodes branch
                let next_block = self.add_to_branch(from, block);
                
                // Request the next block. If there's no more, complete the branch
                if next_block.is_some() {
                    manager.send_to(Packet::BlockRequest(next_block.unwrap()), |x| x == from)?;
                } else {
                    self.complete_branch(from)?;
                }
            },

            BlockChainAddResult::Duplicate => 
            {
                debug!("[{}] Duplicate block {}", self.port, block.header.block_id);
                self.complete_branch(from)?;
            },
        }

        Ok(())
    }

    fn handle_block_request(&mut self, manager: &mut ClientManager, 
                            from: &str, id: u64)
        -> Result<(), Box<dyn Error>>
    {
        info!("Got request for block {}", id);

        let block_or_none = self.chain.block(id);
        if block_or_none.is_some() 
        {
            let block = block_or_none.unwrap();
            manager.send_to(Packet::Block(block.clone()), |x| x == from)?;
        }

        Ok(())
    }

    fn handle_transfer(&mut self, manager: &mut ClientManager, from: &str,
                       transfer: Transaction<Transfer>)
        -> Result<(), Box<dyn Error>>
    {
        info!("Got transfer {:?}", transfer);

        self.chain.push_transfer_queue(transfer.clone())?;
        manager.send_to(
            Packet::Transfer(transfer), 
            |x| x != from)?;

        Ok(())
    }

    fn handle_page(&mut self, manager: &mut ClientManager, from: &str,
                   page: Transaction<Page>, data: DataUnit)
        -> Result<(), Box<dyn Error>>
    {
        info!("Got page {:?}", page);
        
        page.header.content.is_data_valid(&data)?;
        self.chain.push_page_queue(page.clone())?;
        self.data_store.store_data_unit(&data)?;

        manager.send(Packet::Report(None, self.our_report()?))?;
        manager.send_to(
            Packet::Page(page, data),
            |x| x != from)?;

        Ok(())
    }

    fn handle_report(&mut self, manager: &mut ClientManager, 
                     from: &str, report: NodeReport)
        -> Result<(), Box<dyn Error>>
    {
        if self.report.add(from, report.clone()) 
        {
            info!("Got new report for node '{}'", from);
            manager.send_to(Packet::Report(Some(from.to_owned()), report),
                |addr| addr != from)?;
        }

        Ok(())
    }

    fn update_reports(&mut self, _manager: &mut ClientManager)
    {
        for address in self.report.update()
        {
            info!("Report for {} expired", address);
            // TODO: Request new reports
        }
    }

}

#[cfg(test)]
pub mod tests
{

    use super::*;
    use super::packet_handler::NodePacketHandler;
    use crate::network::NetworkConnection;
    use libhyperchain::wallet::private_wallet::PrivateWallet;
    use libhyperchain::block::Block;
    use libhyperchain::miner;

    use std::time::Duration;

    pub fn wait_for_block(connection: &NetworkConnection<NodePacketHandler>, block_id: u64) 
        -> Block
    {
        loop
        {
            {
                let mut node = connection.handler().node();
                let block_or_none = node.chain().block(block_id);
                if block_or_none.is_some() {
                    return block_or_none.unwrap().clone();
                }
            }

            std::thread::sleep(Duration::from_millis(100));
        }
    }

    pub fn create_node(port: u16) -> NetworkConnection<NodePacketHandler>
    {
        let time = libhyperchain::block::current_timestamp();
        let path = std::env::temp_dir().join(format!("{}{}", time, port.to_string()));
        let node = Node::new(port, &path).unwrap();
        let handler = NodePacketHandler::new(node);
        let network_connection = NetworkConnection::open(port, &path, handler).unwrap();
        network_connection
    }

    pub fn mine_block(connection: &mut NetworkConnection<NodePacketHandler>,
                  wallet: &PrivateWallet) -> Block
    {
        let block = 
        {
            let mut node = connection.handler().node();
            let chain = &mut node.chain();
            let block = miner::mine_block(Block::new_blank(chain, wallet)
                .expect("Create block"));

            chain.add(&block).unwrap();
            block
        };

        connection.manager().send(Packet::Block(block.clone())).unwrap();
        block
    }

    #[test]
    fn test_node_branched_chain()
    {
        let wallet = PrivateWallet::open_temp(0).unwrap();

        let mut connection_a = create_node(8030);
        let block_a = mine_block(&mut connection_a, &wallet);
        let block_b = mine_block(&mut connection_a, &wallet);
        let block_c = mine_block(&mut connection_a, &wallet);
        mine_block(&mut connection_a, &wallet);
        mine_block(&mut connection_a, &wallet);

        let mut connection_b = create_node(8031);
        {
            connection_b.handler().node().chain().add(&block_a).unwrap();
            connection_b.handler().node().chain().add(&block_b).unwrap();
            connection_b.handler().node().chain().add(&block_c).unwrap();
        }
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        let block_h_b = mine_block(&mut connection_b, &wallet);

        connection_b.manager().register_node("127.0.0.1:8030");
        let block_h_a = wait_for_block(&connection_a, 7);
        assert_eq!(block_h_a, block_h_b);
    }

    #[test]
    fn test_node_join_with_longer_chain()
    {
        let wallet = PrivateWallet::open_temp(0).unwrap();

        let mut connection_a = create_node(8020);
        mine_block(&mut connection_a, &wallet);
        mine_block(&mut connection_a, &wallet);

        let mut connection_b = create_node(8021);
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        mine_block(&mut connection_b, &wallet);
        let block_d_on_b = mine_block(&mut connection_b, &wallet);
        
        connection_b.manager().register_node("127.0.0.1:8020");
        let block_d_on_a = wait_for_block(&connection_a, 3);
        assert_eq!(block_d_on_a, block_d_on_b);
    }

    #[test]
    fn test_node()
    {
        let wallet = PrivateWallet::open_temp(0).unwrap();

        let mut connection_a = create_node(8010);
        let mut connection_b = create_node(8011);
        connection_b.manager().register_node("127.0.0.1:8010");

        // Transfer block a -> b
        let block_a_on_a = mine_block(&mut connection_a, &wallet);
        let block_a_on_b = wait_for_block(&connection_b, 0);
        assert_eq!(block_a_on_a, block_a_on_b);

        // Transfer 3 blocks b -> a
        let block_b_on_b = mine_block(&mut connection_b, &wallet);
        let block_c_on_b = mine_block(&mut connection_b, &wallet);
        let block_d_on_b = mine_block(&mut connection_b, &wallet);
        let block_e_on_b = mine_block(&mut connection_b, &wallet);
        let block_b_on_a = wait_for_block(&connection_a, 1);
        let block_c_on_a = wait_for_block(&connection_a, 2);
        let block_d_on_a = wait_for_block(&connection_a, 3);
        let block_e_on_a = wait_for_block(&connection_a, 4);
        assert_eq!(block_b_on_b, block_b_on_a);
        assert_eq!(block_c_on_b, block_c_on_a);
        assert_eq!(block_d_on_b, block_d_on_a);
        assert_eq!(block_e_on_b, block_e_on_a);

        // New node joins with a different, shorter chain
        let mut connection_c = create_node(8012);
        mine_block(&mut connection_c, &wallet);
        mine_block(&mut connection_c, &wallet);
        mine_block(&mut connection_c, &wallet);
        connection_c.manager().register_node("127.0.0.1:8010");
        let block_e_on_c = wait_for_block(&connection_c, 4);
        assert_eq!(block_e_on_c, block_e_on_a);

        // New node joins with a different, longer chain
        let mut connection_d = create_node(8013);
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        mine_block(&mut connection_d, &wallet);
        let block_f_on_d = mine_block(&mut connection_d, &wallet);

        connection_d.manager().register_node("127.0.0.1:8010");
        let block_f_on_a = wait_for_block(&connection_a, 5);
        assert_eq!(block_f_on_a, block_f_on_d);
    }

}

