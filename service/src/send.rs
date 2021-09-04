use crate::node::network::NetworkConnection;
use crate::node::Node;
use crate::node::network::Packet;

use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use std::io::Write;

pub fn send<W>(connection: &mut NetworkConnection<Node<W>, W>, 
               from: Vec<u8>, to: Vec<u8>, amount: f32, fee: f32) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    let transaction;
    let transaction_id;

    {
        let from_wallet_or_error = PrivateWallet::deserialize(from);
        if from_wallet_or_error.is_err() {
            return Response::Failed;
        }
        
        let from_wallet = from_wallet_or_error.unwrap();
        let to_address = slice_as_array!(&to, [u8; 32]).unwrap();

        let chain = &mut connection.handler().chain();
        let transaction_or_error = chain.new_transaction(&from_wallet, *to_address, amount, fee);
        if transaction_or_error.is_err() || transaction_or_error.as_ref().unwrap().is_none() {
            return Response::Failed;
        }

        transaction = transaction_or_error.unwrap().unwrap();
        transaction_id = transaction.header.hash().unwrap();
        assert_eq!(chain.push_transaction_queue(transaction.clone()), true);
    }

    connection.manager().send(Packet::Transaction(transaction));
    Response::Sent(transaction_id)
}
