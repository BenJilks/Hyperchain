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

    // Search pending
    let transaction_in_queue = chain.find_transaction_in_queue(&transaction_id_hash);
    if transaction_in_queue.is_some() {
        return Response::TransactionInfo(transaction_in_queue.unwrap(), None);
    }

    // Search chain
    let transaction_in_chain = chain.find_transaction_in_chain(&transaction_id_hash);
    if transaction_in_chain.is_some() 
    {
        let (transaction, block) = transaction_in_chain.unwrap();
        return Response::TransactionInfo(transaction, Some(block));
    }

    Response::Failed
}

