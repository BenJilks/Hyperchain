use super::Command;
use crate::wallet::{PrivateWallet, PublicWallet, Wallet};
use crate::node::network::{NetworkConnection, Packet};
use crate::block::{Page, Block, BlockChain};

use std::sync::{Arc, Mutex};
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;

pub struct PageCommand
{
    page_queue: Vec<Page>,
}

impl Default for PageCommand
{

    fn default() -> Self
    {
        Self
        {
            page_queue: Vec::new(),
        }
    }

}

impl Command for PageCommand
{

    fn name(&self) -> &'static str { "page" }

    fn invoke(&mut self, args: &[String], connection: &mut Arc<Mutex<NetworkConnection>>, chain: &mut BlockChain)
    {
        println!("{:?}", args);
        let owner = PrivateWallet::read_from_file(&PathBuf::from(&args[0])).unwrap();
        let name = args[1].clone();
        let fee = args[2].parse::<f64>().unwrap();

        let mut data = Vec::<u8>::new();
        File::open(&PathBuf::from(&args[3])).unwrap()
            .read_to_end(&mut data).unwrap();

        let page = Page::from_file(chain.longest_branch(), &data, &owner, &name, fee);
        NetworkConnection::broadcast(connection, None, 
            Packet::PageRequest(page.clone()));
        
        self.page_queue.push(page);
    }

    fn on_packet(&mut self, packet: Packet, connection: &mut Arc<Mutex<NetworkConnection>>, chain: &mut BlockChain)
    {
        match packet
        {
            Packet::PageRequest(page) =>
            {
                println!("Got page {}", page.to_string());
                let wallet = PublicWallet::from_public_key_e(page.header.site_id, page.e);
                let balance = wallet.calculate_balance(chain.longest_branch());
        
                // FIXME: Check if we can apply the diff
                if balance < page.header.page_fee
                    || !wallet.varify(&page.header.hash().unwrap(), &page.signature)
                {
                    NetworkConnection::broadcast(connection, None, Packet::PageRequestRejected(page));
                    return;
                }
        
                self.page_queue.push(page.clone());
                NetworkConnection::broadcast(connection, None, Packet::PageRequestAccepted(page));
            },

            Packet::PageRequestAccepted(page) => println!("Accepted {}", page.to_string()),
            Packet::PageRequestRejected(page) => println!("Rejected {}", page.to_string()),
            _ => {},
        }
    }

    fn on_create_block(&mut self, block: &mut Block) 
    {
        for page in &self.page_queue 
        {
            println!("Adding {} to block {}", page.to_string(), block.block_id);
            block.add_page(page.clone());
        }
    }

    fn on_accepted_block(&mut self, block: &Block) 
    {
        for page in &block.pages
        {
            let index = self.page_queue.iter().position(|x| x == page);
            if index.is_some() 
            {
                println!("Got {} in block {}", page.to_string(), block.block_id);
                self.page_queue.remove(index.unwrap());
            }
        }
    }

}
