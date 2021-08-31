extern crate libhyperchain;
extern crate clap;

use libhyperchain::command::{Command, Response};
use libhyperchain::client::Client;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use libhyperchain::logger::{Logger, LoggerLevel, StdLoggerOutput};
use clap::{App, Arg, SubCommand, ArgMatches};
use std::error::Error;
use std::path::PathBuf;

fn balance(mut client: Client, options: &ArgMatches) -> Result<(), Box<dyn Error>>
{
    let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Info);
    let wallet_path = options.value_of("wallet").unwrap();

    let wallet_or_error = PrivateWallet::read_from_file(&PathBuf::from(wallet_path), &mut logger);
    if wallet_or_error.is_err() 
    {
        println!("Error: Unable to open wallet");
        return Ok(());
    }

    let wallet = wallet_or_error.unwrap();
    match client.send(Command::Balance(wallet.as_public()))?
    {
        Response::WalletStatus(status) => println!("Balance: {}", status.balance),
        _ => {},
    }
    Ok(())
}

fn stats(_client: Client)
{
}

fn shutdown(mut client: Client) -> Result<(), Box<dyn Error>>
{
    match client.send(Command::Exit)?
    {
        Response::Exit => println!("Shutdown successfully"),
        _ => {},
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>>
{
    let matches = App::new("Hyperchain Cli")
        .version("0.1.0")
        .author("Ben Jilks <benjyjilks@gmail.com>")
        .about("Command line interface for the hyperchain service")
        .subcommand(SubCommand::with_name("balance")
            .about("Display a wallets balance")
                .arg(Arg::with_name("wallet")
                    .short("w")
                    .long("wallet")
                    .takes_value(true)
                    .required(true)
                    .help("Path to wallet file")))
        .subcommand(SubCommand::with_name("stats")
            .about("Display some blockchain statistics"))
        .subcommand(SubCommand::with_name("shutdown")
            .about("Shutdown service"))
        .get_matches();

    let client_or_error = Client::new();
    if client_or_error.is_err() 
    {
        println!("Error: Could not connect to service");
        return Ok(());
    }

    let client = client_or_error.unwrap();
    match matches.subcommand_name()
    {
        Some("balance") => balance(client, matches.subcommand().1.unwrap())?,
        Some("stats") => stats(client),
        Some("shutdown") => shutdown(client)?,
        Some(&_) | None => println!("Error: Must specify an action"),
    }

    Ok(())
}

