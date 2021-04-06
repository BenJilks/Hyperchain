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
use wallet::PrivateWallet;
use block::{Block, BlockChain};
use std::path::PathBuf;
use node::Node;

fn main()
{
    println!("Hello, Blockchains!!");

    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();

    if std::env::args().len() <= 1
    {
        let mut chain = BlockChain::new(PathBuf::from("blockchain_a"));
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();
        println!("N4L8: {}", chain.longest_branch().lockup_wallet_status(&wallet).balance);
        println!("other: {}", chain.longest_branch().lockup_wallet_status(&other).balance);

        let top_index = chain.top_id();
        let mut block = chain.longest_branch().block(top_index - 2).unwrap();
        block.timestamp += 1;
        block.pow = 0;
        chain.add(&miner::mine_block(block)).unwrap();

        //Node::new(8585, PathBuf::from("known_nodes_a.json"))
        //    .run(&mut chain, &wallet);
    }
    else
    {
        let mut chain = BlockChain::new(PathBuf::from("blockchain_b"));
        let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();
        println!("N4L8: {}", chain.longest_branch().lockup_wallet_status(&wallet).balance);
        println!("other: {}", chain.longest_branch().lockup_wallet_status(&other).balance);

        let mut node = Node::new(8686, PathBuf::from("known_nodes_b.json"));
        node.add_known_node("127.0.0.1:8585");
        node.run(&mut chain, &wallet);
    }
}
