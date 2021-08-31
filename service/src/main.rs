extern crate libhyperchain;

#[macro_use]
extern crate slice_as_array;

mod node;
mod service;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>>
{
    service::start()?;
    Ok(())
}

