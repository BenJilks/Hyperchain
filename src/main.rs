#[macro_use] extern crate slice_as_array;
mod block;
mod miner;
mod wallet;
use wallet::Wallet;
use std::path::PathBuf;
use block::{Block, Transaction, BlockChain};

fn main()
{
    println!("Hello, Blockchains!!");

    let mut chain = BlockChain::new(PathBuf::from("blockchain"));
    let wallet = Wallet::read_from_file(&PathBuf::from("N4L8.wallet")).unwrap();
    let other = Wallet::read_from_file(&PathBuf::from("other.wallet")).unwrap();

    if true
    {
        let mut block = Block::from_chain(&chain, wallet.get_public_key()).unwrap();
        block.add_transaction(Transaction::for_block(&chain, &wallet, other.get_public_key(), 50, 3).unwrap());
        block.add_transaction(Transaction::for_block(&chain, &wallet, other.get_public_key(), 6, 3).unwrap());
        miner::mine_block(&mut chain, block);
    }

    let top = chain.top().unwrap();
    println!("{:?}", top.validate(&chain));
    println!("Balance N4L8: {}", wallet.calculate_balance(&chain));
    println!("Balance Other: {}", other.calculate_balance(&chain));
}
