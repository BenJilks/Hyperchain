use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::service::command::Response;
use libhyperchain::config::HASH_LEN;

pub fn transaction_history(connection: &mut NetworkConnection<Node>,
                           address_vec: Vec<u8>) -> Response
{
    let address_or_none = slice_as_array!(&address_vec, [u8; HASH_LEN]);
    if address_or_none.is_none() {
        return Response::Failed;
    }

    let chain = connection.handler().chain();
    let address = address_or_none.unwrap();
    let transactions = chain.get_transaction_history(address);
    Response::TransactionHistory(transactions)
}
