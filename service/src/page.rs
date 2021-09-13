use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::config::HASH_LEN;

pub fn page_updates(connection: &mut NetworkConnection<NodePacketHandler>,
                    address_vec: Vec<u8>) -> Response
{
    let mut node = connection.handler().node();
    let chain = node.chain();
    let address_or_none = slice_as_array!(&address_vec, [u8; HASH_LEN]);
    if address_or_none.is_none() {
        return Response::Failed;
    }

    let address = address_or_none.unwrap();
    Response::PageUpdates(chain.get_page_updates(address))
}

pub fn page_data(connection: &mut NetworkConnection<NodePacketHandler>, 
                 transaction_id: Vec<u8>) -> Response
{
    match connection.handler().node().data_store().get(&transaction_id)
    {
        Ok(data) => Response::PageData(data),
        Err(_) => Response::Failed,
    }
}

