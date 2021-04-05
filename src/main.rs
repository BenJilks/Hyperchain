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
use std::env;

fn main()
{
    println!("Hello, Blockchains!!");

    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();

    if env::args().len() <= 1
    {
        let mut chain = BlockChain::new(PathBuf::from("blockchain_a"));
        Node::new(8585, PathBuf::from("known_nodes_a"))
            .run(&mut chain, &wallet);
    }
    else
    {
        let mut chain = BlockChain::new(PathBuf::from("blockchain_b"));
        let mut node = Node::new(8686, PathBuf::from("known_nodes_b"));
        node.add_known_node("127.0.0.1:8585");
        node.run(&mut chain, &wallet);
    }

    //let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();
    /*
    if true
    {
        let mut block = Block::new(&chain, &other).unwrap();
        block.add_page(Page::from_file(&chain, "<video src=\"dogecoin.mp4\" autoplay loop/>".as_bytes(), &other, "index.html", 1));
        miner::mine_block(&mut chain, block);
    }

    let top = chain.top().unwrap();
    println!("{:?}", top);
    println!("Balance N4L8: {}", wallet.calculate_balance(&chain));
    println!("Balance Other: {}", other.calculate_balance(&chain));
    */
}
