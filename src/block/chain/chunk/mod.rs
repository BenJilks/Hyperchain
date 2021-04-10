mod main_chunk;
mod sub_chunk;
use crate::block::Block;
use crate::error::Error;
pub use main_chunk::MainChunk;
pub use sub_chunk::SubChunk;

use std::path::PathBuf;

pub const CHUNK_SIZE: u64 = 10;

pub trait Chunk
{

    fn from(path: PathBuf, chunk_id: u64) -> Self;
    fn write(&self, path: PathBuf);

    fn block(&self, block_id: u64) -> Option<Block>;
    fn set_block(&mut self, block: Block) -> Result<(), Error>;

}
