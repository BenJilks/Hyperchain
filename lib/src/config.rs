
pub const BLOCK_SIZE: usize = 16 * 1024 * 1024; // 16 MB
pub const BLOCK_TIME: u64 = 1000;
pub const BLOCK_SAMPLE_SIZE: u64 = 10;
pub const PAGE_CHUNK_SIZE: usize = 1000 * 1000; // 1MB

pub const PUB_KEY_LEN: usize = 256;
pub const HASH_LEN: usize = 32;
pub type Signature = [u8; PUB_KEY_LEN];
pub type Hash = [u8; HASH_LEN];
