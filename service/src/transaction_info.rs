use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::config::HASH_LEN;

pub fn transaction_info(connection: &mut NetworkConnection<NodePacketHandler>,
                        transaction_id: Vec<u8>) 
    -> Response
{
    let transaction_id_hash_or_none = slice_as_array!(&transaction_id, [u8; HASH_LEN]);
    if transaction_id_hash_or_none.is_none() {
        return Response::Failed;
    }

    let mut node = connection.handler().node();
    let transaction_id_hash = transaction_id_hash_or_none.unwrap();
    let chain = node.chain();

    let transaction_or_none = chain.find_transaction(&transaction_id_hash);
    if transaction_or_none.is_none() {
        return Response::Failed;
    }

    let (transaction, block) = transaction_or_none.unwrap();
    Response::TransactionInfo(transaction, block)
}

