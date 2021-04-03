use crate::block::{Block, BlockChain};
use crate::wallet::PrivateWallet;

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
    let mut block = Block::new(chain, wallet)?;
    let mut blocks_found = 0;
    if !block.validate(chain)
    {
        println!("Block is not valid!!");
        return None;
    }

    while blocks_found < blocks_to_mine
    {
        if block.validate_pow()
        {
            println!("Block {} in found in {}!!", block.block_id, block.pow);
            chain.add(&block).unwrap();
            blocks_found += 1;

            block = Block::new(chain, wallet)?;
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
