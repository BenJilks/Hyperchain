use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::service::command::Response;
use libhyperchain::block::HASH_LEN;
use libhyperchain::chain::transaction_queue::BlockChainTransactionQueue;
use std::io::Write;

pub fn transaction_info<W>(connection: &mut NetworkConnection<Node<W>, W>, 
               transaction_id: Vec<u8>) -> Response
    where W: Write + Clone + Sync + Send + 'static
{
    let transaction_id_hash_or_none = slice_as_array!(&transaction_id, [u8; HASH_LEN]);
    if transaction_id_hash_or_none.is_none() {
        return Response::Failed;
    }

    let transaction_id_hash = transaction_id_hash_or_none.unwrap();
    let chain = connection.handler().chain();

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

