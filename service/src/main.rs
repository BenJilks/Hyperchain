extern crate libhyperchain;

mod node;
mod service;

use std::error::Error;

fn main() -> Result<(), Box<dyn Error>>
{
    service::start()?;
    Ok(())
}

