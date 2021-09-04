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
    let transfer;
    let transfer_id;

    {
        let from_wallet_or_error = PrivateWallet::deserialize(from);
        if from_wallet_or_error.is_err() {
            return Response::Failed;
        }
        
        let from_wallet = from_wallet_or_error.unwrap();
        let to_address = slice_as_array!(&to, [u8; 32]).unwrap();

        let chain = &mut connection.handler().chain();
        let transfer_or_error = chain.new_transfer(&from_wallet, *to_address, amount, fee);
        if transfer_or_error.is_err() || transfer_or_error.as_ref().unwrap().is_none() {
            return Response::Failed;
        }

        transfer = transfer_or_error.unwrap().unwrap();
        transfer_id = transfer.header.hash().unwrap();
        assert_eq!(chain.push_transaction_queue(transfer.clone()), true);
    }

    connection.manager().send(Packet::Transfer(transfer));
    Response::Sent(transfer_id)
}
