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
extern crate serde_json;

mod block;
mod miner;
mod wallet;
mod error;
mod logger;
mod node;
// use wallet::PrivateWallet;
// use std::path::PathBuf;
// use logger::{Logger, LoggerLevel, StdLoggerOutput};
// use block::BlockChain;
// use node::Node;
// use node::network::{NetworkConnection, Packet};
// use std::sync::{Arc, Mutex};

fn main()
{
    println!("Hello, Blockchains!!");
}

