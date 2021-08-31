use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::command::Response;
use libhyperchain::wallet::public_wallet::PublicWallet;
use libhyperchain::wallet::Wallet;
use std::io::Write;

pub fn balance<W>(network_connection: &mut NetworkConnection<Node<W>, W>, 
              wallet: PublicWallet) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    let chain = network_connection.handler().chain();
    let status = wallet.get_status(chain);
    Response::WalletStatus(status)
}

