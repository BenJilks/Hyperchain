#[macro_use] 
extern crate slice_as_array;

#[cfg(feature = "serde_derive")] 
#[macro_use]
extern crate serde;

#[macro_use]
extern crate serde_big_array;

extern crate sha2;
extern crate bincode;
extern crate rsa;
extern crate rand;
extern crate num_traits;
extern crate bidiff;
extern crate bipatch;
extern crate base_62;
extern crate serde_json;

mod block;
mod miner;
mod wallet;
mod node;
mod error;
mod logger;
use wallet::PrivateWallet;
use block::BlockChain;
use logger::{Logger, LoggerLevel};
use std::path::PathBuf;
use node::Node;

fn main()
{
    println!("Hello, Blockchains!!");

    let mut logger = Logger::new(std::io::stdout(), LoggerLevel::Info);
    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet"), &mut logger).unwrap();
    let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet"), &mut logger).unwrap();
    
    if std::env::args().len() <= 1
    {
        let mut chain = BlockChain::new(PathBuf::from("blockchain_a"), &mut logger);
        println!("N4L8: {}", chain.lockup_wallet_status(&wallet).balance);
        println!("other: {}", chain.lockup_wallet_status(&other).balance);

        let mut node = Node::new(8585, PathBuf::from("known_nodes_a.json"));
        node.add_known_node("127.0.0.1:8686");
        node.run(&mut chain, &wallet, &mut logger);
    }
    else
    {
        let mut chain = BlockChain::new(PathBuf::from("blockchain_b"), &mut logger);
        println!("N4L8: {}", chain.lockup_wallet_status(&wallet).balance);
        println!("other: {}", chain.lockup_wallet_status(&other).balance);

        let mut node = Node::new(8686, PathBuf::from("known_nodes_b.json"));
        node.add_known_node("pi:8585");
        node.run(&mut chain, &other, &mut logger);
    }
}
