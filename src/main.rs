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
use block::BlockChain;
use std::path::PathBuf;
use node::Node;

fn main()
{
    println!("Hello, Blockchains!!");

    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();
    let mut chain = BlockChain::new(PathBuf::from("blockchain"));
    Node::new(8585, PathBuf::from("known_nodes.json"))
        .run(&mut chain, &wallet);
}
