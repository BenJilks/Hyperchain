use crate::network::NetworkConnection;
use crate::network::packet::Packet;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;

fn deserialize_inputs(serilized_inputs: Vec<(Vec<u8>, f32)>) 
    -> Option<Vec<(PrivateWallet, f32)>>
{
    let mut inputs = Vec::new();
    for (from, amount) in serilized_inputs
    {
        let from_wallet_or_error = PrivateWallet::deserialize(from);
        if from_wallet_or_error.is_err() {
            return None;
        }
        
        let from_wallet = from_wallet_or_error.unwrap();
        inputs.push((from_wallet, amount));
    }

    Some(inputs)
}

pub fn send(connection: &mut NetworkConnection<NodePacketHandler>,
            serilized_inputs: Vec<(Vec<u8>, f32)>, to: Vec<u8>, amount: f32, fee: f32)
    -> Response
{
    let transfer;
    let transfer_id;

    {
        let inputs_or_none = deserialize_inputs(serilized_inputs);
        if inputs_or_none.is_none() {
            return Response::Failed;
        }

        let to_address = slice_as_array!(&to, [u8; 32]).unwrap();
        let inputs = inputs_or_none.unwrap();
        let ref_inputs = inputs.iter().map(|(w, a)| (w, *a)).collect::<Vec<_>>();
        
        let mut node = connection.handler().node();
        let chain = &mut node.chain();
        let transfer_or_error = chain.new_transfer(ref_inputs, *to_address, amount, fee);
        if transfer_or_error.is_err() || transfer_or_error.as_ref().unwrap().is_none() {
            return Response::Failed;
        }

        transfer = transfer_or_error.unwrap().unwrap();
        transfer_id = transfer.hash().unwrap();
        if !chain.push_transfer_queue(transfer.clone()) {
            return Response::Failed;
        }
    }

    connection.manager().send(Packet::Transfer(transfer)).unwrap();
    Response::Sent(transfer_id)
}

