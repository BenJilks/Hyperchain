use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::hash::Hash;

pub fn balance(connection: &mut NetworkConnection<NodePacketHandler>,
               address_vec: Vec<u8>) -> Response
{
    let mut node = connection.handler().node();
    let chain = node.chain();

    // TODO: Varify this is a valid hash
    let address = Hash::from(&address_vec);

    let status = chain.get_wallet_status(&address);
    Response::WalletStatus(status)
}

