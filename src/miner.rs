use crate::block::{Block, BlockChain};
use crate::wallet::Wallet;

pub fn mine_block(chain: &mut BlockChain, mut block: Block)
{    
    println!("Started mining");
    loop
    {
        if block.validate(chain)
        {
            println!("Block {} found!!", block.block_id);
            chain.add(&block).unwrap();
            break;
        }

        block.pow += 1;
    }
}

pub fn mine(chain: &mut BlockChain, wallet: &Wallet, blocks_to_mine: i32) -> Option<()>
{    
    let for_pub_key = wallet.get_public_key();

    let mut prev: Option<&Block> = None;
    let mut last_block = Block::new(None, for_pub_key)?;
    let top_or_none = chain.top();
    if top_or_none.is_some() 
    {
        last_block = top_or_none.unwrap();
        println!("Found top {}", last_block.block_id);
        prev = Some( &last_block );
    }

    let mut block = Block::new(prev, for_pub_key)?;
    let mut blocks_found = 0;
    while blocks_found < blocks_to_mine
    {
        if block.validate(chain)
        {
            println!("Block {} found!!", block.block_id);
            chain.add(&block).unwrap();
            blocks_found += 1;

            last_block = block.clone();
            prev = Some( &last_block );
            block = Block::new(prev, for_pub_key)?;
            continue;
        }

        block.pow += 1;
    }

    Some(())
}
