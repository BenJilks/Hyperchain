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

mod block;
mod miner;
mod wallet;
use wallet::{PrivateWallet, Wallet};
use std::path::PathBuf;
use block::{Block, Transaction, BlockChain};

fn main()
{
    println!("Hello, Blockchains!!");

    let mut chain = BlockChain::new(PathBuf::from("blockchain"));
    let wallet = PrivateWallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();
    let other = PrivateWallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();

    miner::mine(&mut chain, &wallet, 25).unwrap();

    if true
    {
        let mut block = Block::new(&chain, other.get_public_key()).unwrap();
        block.add_transaction(Transaction::for_block(&chain, &wallet, other.get_public_key(), 40, 3).unwrap());
        block.add_transaction(Transaction::for_block(&chain, &wallet, other.get_public_key(), 6, 1).unwrap());
        miner::mine_block(&mut chain, block);
    }

    let top = chain.top().unwrap();
    println!("{:?}", top);
    println!("Balance N4L8: {}", wallet.calculate_balance(&chain));
    println!("Balance Other: {}", other.calculate_balance(&chain));
}
