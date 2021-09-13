use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::service::command::Response;
use libhyperchain::config::HASH_LEN;

pub fn balance(network_connection: &mut NetworkConnection<Node>,
               address_vec: Vec<u8>) -> Response
{
    let chain = network_connection.handler().chain();
    let address = slice_as_array!(&address_vec, [u8; HASH_LEN]);
    if address.is_none() {
        return Response::Failed;
    }

    let status = chain.get_wallet_status(&address.unwrap());
    Response::WalletStatus(status)
}
