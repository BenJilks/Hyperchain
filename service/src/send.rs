use crate::network::NetworkConnection;
use crate::network::packet::Packet;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use libhyperchain::hash::Hash;

fn deserialize_inputs(serialized_inputs: Vec<(Vec<u8>, f32)>) 
    -> Option<Vec<(PrivateWallet, f32)>>
{
    let mut inputs = Vec::new();
    for (from, amount) in serialized_inputs
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

fn deserialize_outputs(serialized_outputs: Vec<(Vec<u8>, f32)>)
    -> Option<Vec<(Hash, f32)>>
{
    let mut outputs = Vec::new();
    for (to_vec, amount) in serialized_outputs
    {
        // TODO: Varify this is a valid hash
        let to = Hash::from(&to_vec);
        outputs.push((to, amount));
    }

    Some(outputs)
}

pub fn send(connection: &mut NetworkConnection<NodePacketHandler>,
            serialized_inputs: Vec<(Vec<u8>, f32)>,
            serialized_outputs: Vec<(Vec<u8>, f32)>,
            fee: f32)
    -> Response
{
    let transfer;
    let transfer_id;

    {
        let inputs_or_none = deserialize_inputs(serialized_inputs);
        if inputs_or_none.is_none() {
            return Response::Failed;
        }

        let outputs_or_none = deserialize_outputs(serialized_outputs);
        if outputs_or_none.is_none() {
            return Response::Failed;
        }

        let outputs = outputs_or_none.unwrap();
        let inputs = inputs_or_none.unwrap();
        let ref_inputs = inputs.iter().map(|(w, a)| (w, *a)).collect::<Vec<_>>();
        
        let mut node = connection.handler().node();
        let chain = &mut node.chain();
        let transfer_or_error = chain.new_transfer(ref_inputs, outputs, fee);
        if transfer_or_error.is_err() 
        {
            warn!("Error in send: {}", transfer_or_error.unwrap_err());
            return Response::Failed;
        }

        transfer = transfer_or_error.unwrap();
        transfer_id = transfer.hash().unwrap();
        let result = chain.push_transfer_queue(transfer.clone());
        if result.is_err() 
        {
            warn!("Error in send: {}", result.unwrap_err());
            return Response::Failed;
        }
    }

    connection.manager().send(Packet::Transfer(transfer)).unwrap();
    Response::Sent(transfer_id.data().to_vec())
}

