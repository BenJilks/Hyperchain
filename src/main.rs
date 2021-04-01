#[macro_use] extern crate slice_as_array;
mod block;
use block::{Block, BlockChain};
use std::path::PathBuf;

fn miner() -> Option<()>
{    
    println!("Hello, Blockchains!!");

    let chain = BlockChain::new(PathBuf::from("blockchain"));

    let mut prev: Option<&Block> = None;
    let mut last_block = Block::new(None)?;
    let top_or_none = chain.top();
    if top_or_none.is_some() 
    {
        last_block = top_or_none.unwrap();
        println!("Found top {}", last_block.block_id);
        prev = Some( &last_block );
    }

    let mut block = Block::new(prev)?;
    let mut blocks_found = 0;
    while blocks_found <= 20
    {
        if block.validate(prev)
        {
            println!("Block {} found!!", block.block_id);
            chain.add(&block).unwrap();

            last_block = block.clone();
            prev = Some( &last_block );
            block = Block::new(prev)?;
            blocks_found += 1;
            continue;
        }

        block.pow += 1;
    }

    Some(())
}

fn main()
{
    miner().unwrap();
}
