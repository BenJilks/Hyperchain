extern crate libhyperchain;

use libhyperchain::{Client, Command};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>>
{
    let mut client = Client::new()?;
    client.send(Command::Exit)?;

    Ok(())
}
