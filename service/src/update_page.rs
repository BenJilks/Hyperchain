use crate::node::network::NetworkConnection;
use crate::node::Node;
// use crate::node::network::Packet;

use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use std::io::Write;

pub fn update_page<W>(connection: &mut NetworkConnection<Node<W>, W>, 
                      from: Vec<u8>) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    let page;
    let page_id;

    {
        let from_wallet_or_error = PrivateWallet::deserialize(from);
        if from_wallet_or_error.is_err() {
            return Response::Failed;
        }
        
        let from_wallet = from_wallet_or_error.unwrap();

        let chain = &mut connection.handler().chain();
        let page_or_error = chain.new_page(&from_wallet, Vec::new(), 0, 1.0);
        if page_or_error.is_err() || page_or_error.as_ref().unwrap().is_none() {
            return Response::Failed;
        }

        page = page_or_error.unwrap().unwrap();
        page_id = page.hash().unwrap();
        assert_eq!(chain.push_page_queue(page.clone()), true);
    }

    // connection.manager().send(Packet::Page(page));
    Response::Sent(page_id)
}
