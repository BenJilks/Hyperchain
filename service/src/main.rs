#[macro_use] 
extern crate slice_as_array;

#[cfg(feature = "serde_derive")] 
#[macro_use]
extern crate serde;

#[macro_use]
extern crate serde_big_array;

extern crate libhyperchain;
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
mod chain;
mod transaction;
mod page;
mod miner;
mod wallet;
mod error;
mod logger;
mod node;
mod service;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>>
{
    service::start()?;
    Ok(())
}
