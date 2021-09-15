extern crate libhyperchain;
extern crate clap;
extern crate base_62;
extern crate pretty_env_logger;

use libhyperchain::service::command::{Command, Response};
use libhyperchain::service::client::Client;
use libhyperchain::wallet::Wallet;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use libhyperchain::transaction::TransactionVariant;
use clap::{App, Arg, SubCommand, ArgMatches};
use std::path::PathBuf;
use std::fs::File;
use std::io::Read;
use std::error::Error;

fn balance(mut client: Client, options: &ArgMatches) -> Result<(), Box<dyn Error>>
{
    let wallet_path = options.value_of("wallet").unwrap();
    let wallet_or_error = PrivateWallet::read_from_file(&PathBuf::from(wallet_path));
    if wallet_or_error.is_err() 
    {
        println!("Error: Unable to open wallet");
        return Ok(());
    }

    let wallet = wallet_or_error.unwrap();
    match client.send(Command::Balance(wallet.get_address().to_vec()))?
    {
        Response::WalletStatus(status) =>
        {
            println!("Address: {}", base_62::encode(&wallet.get_address()));
            println!("Balance: {}", status.balance)
        },
        _ => {},
    }
    Ok(())
}

fn send(mut client: Client, options: &ArgMatches) -> Result<(), Box<dyn Error>>
{
    let from_path = options.value_of("from").unwrap();
    let from_or_error = PrivateWallet::read_from_file(&PathBuf::from(from_path));
    if from_or_error.is_err() 
    {
        println!("Error: Unable to open wallet");
        return Ok(());
    }

    let from = from_or_error.unwrap();
    let to = base_62::decode(options.value_of("to").unwrap())?;
    let amount = options.value_of("amount").unwrap().parse::<f32>()?;
    let fee = options.value_of("fee").unwrap().parse::<f32>()?;
    match client.send(Command::Send(from.serialize(), to, amount, fee))?
    {
        Response::Sent(id) => 
            println!("Success, TxID: {}", base_62::encode(&id)),
        _ => println!("Error"),
    }
    Ok(())
}

fn update_page(mut client: Client, options: &ArgMatches) -> Result<(), Box<dyn Error>>
{
    let from_path = options.value_of("from").unwrap();
    let from_or_error = PrivateWallet::read_from_file(&PathBuf::from(from_path));
    if from_or_error.is_err() 
    {
        println!("Error: Unable to open wallet");
        return Ok(());
    }

    let from = from_or_error.unwrap();
    let name = options.value_of("name").unwrap().to_owned();
    
    let page_path = options.value_of("page").unwrap();
    let mut page = Vec::new();
    File::open(&page_path)?.read_to_end(&mut page)?;

    match client.send(Command::UpdatePage(from.serialize(), name, page))?
    {
        Response::Sent(id) => 
            println!("Success, TxID: {}", base_62::encode(&id)),
        _ => println!("Error"),
    }
    Ok(())
}

fn transaction_info(mut client: Client, options: &ArgMatches) 
    -> Result<(), Box<dyn Error>>
{
    let id = base_62::decode(options.value_of("id").unwrap())?;

    match client.send(Command::TransactionInfo(id))?
    {
        Response::TransactionInfo(transaction, block) => 
        {
            match transaction
            {
                TransactionVariant::Transfer(transfer) =>
                {
                    println!("Transfer:");
                    println!("From: {}", base_62::encode(&transfer.get_from_address()));
                    println!("To: {}", base_62::encode(&transfer.header.to));
                    println!("Amount: {}", transfer.header.amount);
                    println!("Fee: {}", transfer.header.fee);
                },

                TransactionVariant::Page(page) =>
                {
                    println!("Page:");
                    println!("From: {}", base_62::encode(&page.get_from_address()));
                    println!("Length: {} bytes", page.header.data_length);
                    println!("Fee: {}", page.header.fee);
                },
            }
            
            match block
            {
                Some(block) => println!("In block: {}", block.block_id),
                None => println!("In block: Pending"),
            }
        },
        _ => println!("Error"),
    }
    Ok(())
}

fn new_wallet(options: &ArgMatches) -> Result<(), Box<dyn Error>>
{
    let output = options.value_of("output").unwrap();

    println!("Creating new wallet...");
    let wallet = PrivateWallet::new()?;
    wallet.write_to_file(&PathBuf::from(output))?;
    println!("Done!");

    Ok(())
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
    pretty_env_logger::init();

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

        .subcommand(SubCommand::with_name("send")
            .about("Sent coins to someone")
            .arg(Arg::with_name("from")
                .short("f")
                .long("from")
                .takes_value(true)
                .required(true)
                .help("Path to from wallet file"))
            .arg(Arg::with_name("to")
                .short("t")
                .long("to")
                .takes_value(true)
                .required(true)
                .help("Address of recipient"))
            .arg(Arg::with_name("amount")
                    .short("a")
                    .long("amount")
                    .takes_value(true)
                    .required(true)
                    .help("Amount to send"))
            .arg(Arg::with_name("fee")
                    .short("e")
                    .long("fee")
                    .takes_value(true)
                    .required(true)
                    .help("Transaction fee")))
        
        .subcommand(SubCommand::with_name("update-page")
            .about("Update your page")
            .arg(Arg::with_name("from")
                .short("f")
                .long("from")
                .takes_value(true)
                .required(true)
                .help("Path to from wallet file"))
            .arg(Arg::with_name("name")
                .short("n")
                .long("name")
                .takes_value(true)
                .required(true)
                .help("Page path and file name"))
            .arg(Arg::with_name("page")
                .short("p")
                .long("page")
                .takes_value(true)
                .required(true)
                .help("Path to page file")))
        
        .subcommand(SubCommand::with_name("transaction-info")
            .about("Display transaction information")
            .arg(Arg::with_name("id")
                    .short("i")
                    .long("id")
                    .takes_value(true)
                    .required(true)
                    .help("Id of transaction")))
        
        .subcommand(SubCommand::with_name("new-wallet")
            .about("Create a new wallet")
            .arg(Arg::with_name("output")
                 .short("o")
                 .long("output")
                 .takes_value(true)
                 .required(true)
                 .help("Output path of new wallet")))

        .subcommand(SubCommand::with_name("shutdown")
            .about("Shutdown service"))
        
        .get_matches();
    
    match matches.subcommand_name()
    {
        Some("new-wallet") => return new_wallet(matches.subcommand().1.unwrap()),
        Some(&_) | None => {},
    }

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
        Some("send") => send(client, matches.subcommand().1.unwrap())?,
        Some("update-page") => update_page(client, matches.subcommand().1.unwrap())?,
        Some("transaction-info") => transaction_info(client, matches.subcommand().1.unwrap())?,
        Some("shutdown") => shutdown(client)?,
        Some(&_) | None => println!("Error: Must specify an action"),
    }

    Ok(())
}

