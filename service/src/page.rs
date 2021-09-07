use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::service::command::Response;
use libhyperchain::config::HASH_LEN;
use std::io::Write;

pub fn page_updates<W>(connection: &mut NetworkConnection<Node<W>, W>, 
                       address_vec: Vec<u8>) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    let chain = connection.handler().chain();
    let address_or_none = slice_as_array!(&address_vec, [u8; HASH_LEN]);
    if address_or_none.is_none() {
        return Response::Failed;
    }

    let address = address_or_none.unwrap();
    Response::PageUpdates(chain.get_page_updates(address))
}

pub fn page_data<W>(connection: &mut NetworkConnection<Node<W>, W>, 
                    transaction_id: Vec<u8>) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    match connection.handler().data_store().get(&transaction_id)
    {
        Ok(data) => Response::PageData(data),
        Err(_) => Response::Failed,
    }
}
