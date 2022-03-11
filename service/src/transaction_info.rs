use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::hash::Hash;

pub fn transaction_info(connection: &mut NetworkConnection<NodePacketHandler>,
                        transaction_id: Vec<u8>) 
    -> Response
{
    // TODO: Varify this is a valid hash
    let transaction_id_hash = Hash::from(&transaction_id);

    let mut node = connection.handler().node();
    let chain = node.chain();

    let transaction_or_none = chain.find_transaction(&transaction_id_hash);
    if transaction_or_none.is_none() {
        return Response::Failed;
    }

    let (transaction, block) = transaction_or_none.unwrap();
    Response::TransactionInfo(transaction, block)
}

