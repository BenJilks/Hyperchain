use crate::node::network::NetworkConnection;
use crate::node::packet_handler::Packet;
use crate::node::Node;

use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use libhyperchain::transaction::Transaction;
use libhyperchain::transaction::page::Page;
use libhyperchain::data_store::DataUnit;
use libhyperchain::data_store::page::CreatePageData;

fn add_page(connection: &mut NetworkConnection<Node>, from: Vec<u8>, data_unit: &DataUnit)
    -> Option<(Transaction<Page>, Vec<u8>)>
{
    let from_wallet_or_error = PrivateWallet::deserialize(from);
    if from_wallet_or_error.is_err() {
        return None;
    }
    
    let chain = &mut connection.handler().chain();
    let from_wallet = from_wallet_or_error.unwrap();
    let page_or_error = chain.new_page(&from_wallet, &data_unit, 1.0);
    if page_or_error.is_err() || page_or_error.as_ref().unwrap().is_none() {
        return None;
    }

    let page = page_or_error.unwrap().unwrap();
    assert_eq!(chain.push_page_queue(page.clone()), true);
    
    let page_id = page.hash().unwrap();
    Some((page, page_id))
}

pub fn update_page(connection: &mut NetworkConnection<Node>,
                   from: Vec<u8>, name: String, data: Vec<u8>) 
    -> Response
{
    let data_unit = DataUnit::CreatePage(CreatePageData::new(name, data));
    let page_or_none = add_page(connection, from, &data_unit);
    if page_or_none.is_none() {
        return Response::Failed;
    }

    let (page, page_id) = page_or_none.unwrap();
    connection.handler().data_store().store(&page_id, &data_unit).unwrap();
    connection.manager().send(Packet::Page(page, data_unit));
    Response::Sent(page_id)
}
