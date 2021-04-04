use crate::block::{Block, BlockChainBranch};
use crate::wallet::PrivateWallet;

pub fn mine_block(chain: &mut BlockChainBranch, mut block: Block) -> Result<(), String>
{
    println!("Started mining");
    block.validate(chain)?;

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

    Ok(())
}

pub fn mine(chain: &mut BlockChainBranch, wallet: &PrivateWallet, blocks_to_mine: i32) -> Result<(), String>
{    
    let mut block = Block::new(chain, wallet)?;
    let mut blocks_found = 0;
    block.validate(chain)?;

    while blocks_found < blocks_to_mine
    {
        if block.validate_pow()
        {
            println!("Block {} in found in {}!!", block.block_id, block.pow);
            chain.add(&block).unwrap();
            blocks_found += 1;

            block = Block::new(chain, wallet)?;
            block.validate(chain)?;
            continue;
        }

        block.pow += 1;
    }

    Ok(())
}
