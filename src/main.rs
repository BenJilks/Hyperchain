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

mod block;
mod miner;
mod wallet;
use wallet::{PrivateWallet, Wallet};
use std::path::PathBuf;
use block::{Block, Page, BlockChain};

fn main()
{
    println!("Hello, Blockchains!!");

    let mut chain = BlockChain::new(PathBuf::from("blockchain"));
    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();
    let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();

    //miner::mine(&mut chain, &wallet, 25).unwrap();

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
}
