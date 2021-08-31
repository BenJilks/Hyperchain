mod miner;
use miner::start_miner_thread;
use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::chain::BlockChain;
use libhyperchain::logger::{Logger, LoggerLevel, StdLoggerOutput};
use libhyperchain::server;
use libhyperchain::command::{Command, Response};
use libhyperchain::wallet::public_wallet::PublicWallet;
use libhyperchain::wallet::Wallet;
use std::error::Error;
use std::io::Write;

fn balance<W>(network_connection: &mut NetworkConnection<Node<W>, W>, 
              wallet: PublicWallet) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    let chain = network_connection.handler().chain();
    let status = wallet.get_status(chain);
    Response::WalletStatus(status)
}

pub fn start() -> Result<(), Box<dyn Error>>
{
    // Crate logger and read port from command line
    let mut logger = Logger::new(StdLoggerOutput::new(), LoggerLevel::Info);
    let port = std::env::args().nth(1).unwrap().parse::<u16>()?;

    // Create and open node
    let chain = BlockChain::new(&mut logger);
    let node = Node::new(port, chain, logger.clone());

    let miner_thread;
    {
        let network_connection = NetworkConnection::new(port, node, logger.clone());

        // Register a common node to connect to
        network_connection.lock().unwrap().manager().register_node("127.0.0.1:8001", None);
        
        miner_thread = start_miner_thread(network_connection.clone(), logger.clone());

        let connection = network_connection.clone();
        server::start(move |command|
        {
            match command
            {
                Command::Exit => 
                    Response::Exit,

                Command::Balance(wallet) => 
                    balance(&mut connection.lock().unwrap(), wallet),
            }
        })?;

        NetworkConnection::shutdown(&network_connection);
    }

    miner_thread.join().unwrap();
    Ok(())
}
