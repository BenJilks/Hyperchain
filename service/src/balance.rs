use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::config::HASH_LEN;

pub fn balance(connection: &mut NetworkConnection<NodePacketHandler>,
               address_vec: Vec<u8>) -> Response
{
    let mut node = connection.handler().node();
    let chain = node.chain();
    let address = slice_as_array!(&address_vec, [u8; HASH_LEN]);
    if address.is_none() {
        return Response::Failed;
    }

    let status = chain.get_wallet_status(&address.unwrap());
    Response::WalletStatus(status)
}

