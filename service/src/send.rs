use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::transaction::Transaction;
use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use libhyperchain::chain::transaction_queue::BlockChainTransactionQueue;
use std::io::Write;

pub fn send<W>(connection: &mut NetworkConnection<Node<W>, W>, 
               from: Vec<u8>, to: Vec<u8>, amount: f32, fee: f32) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    let chain = &mut connection.handler().chain();

    let from_wallet_or_error = PrivateWallet::deserialize(from);
    if from_wallet_or_error.is_err() {
        return Response::Failed;
    }

    let from_wallet = from_wallet_or_error.unwrap();
    let to_address = slice_as_array!(&to, [u8; 32]).unwrap();
    let transaction_or_none = Transaction::for_chain(chain, &from_wallet, *to_address, amount, fee);
    if transaction_or_none.is_none() {
        return Response::Failed;
    }

    let transaction = transaction_or_none.unwrap();
    let transaction_id = transaction.header.hash().unwrap();
    match chain.push_transaction_queue(transaction)
    {
        Ok(true) => Response::Sent(transaction_id),
        Ok(false) | Err(_) => Response::Failed,
    }
}

