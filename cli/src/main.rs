extern crate libhyperchain;

use libhyperchain::{Client, Command};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>>
{
    let client_or_error = Client::new();
    if client_or_error.is_err() 
    {
        println!("Error: Could not connect to service");
        return Ok(());
    }

    let mut client = client_or_error.unwrap();
    client.send(Command::Exit)?;
    Ok(())
}

