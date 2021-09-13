use crate::node::network::NetworkConnection;
use crate::node::packet_handler::Packet;
use crate::node::Node;

use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;

pub fn send(connection: &mut NetworkConnection<Node>,
               from: Vec<u8>, to: Vec<u8>, amount: f32, fee: f32) -> Response
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
        transfer_id = transfer.hash().unwrap();
        assert_eq!(chain.push_transfer_queue(transfer.clone()), true);
    }

    connection.manager().send(Packet::Transfer(transfer));
    Response::Sent(transfer_id)
}
