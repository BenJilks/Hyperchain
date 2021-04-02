use crate::block::{Block, BlockChain};
use crate::wallet::{PrivateWallet, Wallet};

pub fn mine_block(chain: &mut BlockChain, mut block: Block)
{
    println!("Started mining");
    if !block.validate(chain)
    {
        println!("Block is not valid!!");
        return;
    }

    loop
    {
        if block.validate_pow()
        {
            println!("Block {} found!!", block.block_id);
            chain.add(&block).unwrap();
            break;
        }

        block.pow += 1;
    }
}

pub fn mine(chain: &mut BlockChain, wallet: &PrivateWallet, blocks_to_mine: i32) -> Option<()>
{    
    let for_pub_key = wallet.get_public_key();

    let mut prev: Option<&Block> = None;
    let mut last_block: Block;
    let top_or_none = chain.top();
    if top_or_none.is_some() 
    {
        last_block = top_or_none.unwrap();
        println!("Found top {}", last_block.block_id);
        prev = Some( &last_block );
    }

    let mut block = Block::new(prev, for_pub_key)?;
    let mut blocks_found = 0;
    if !block.validate(chain)
    {
        println!("Block is not valid!!");
        return None;
    }

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
            if !block.validate(chain)
            {
                println!("Block is not valid!!");
                return None;
            }
            continue;
        }

        block.pow += 1;
    }

    Some(())
}
