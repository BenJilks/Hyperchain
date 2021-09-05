use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::transaction::TransactionVariant;
use libhyperchain::service::command::Response;
use libhyperchain::block::Block;
use libhyperchain::config::HASH_LEN;
use std::io::Write;

pub fn transaction_history<W>(connection: &mut NetworkConnection<Node<W>, W>,
                              address_vec: Vec<u8>) -> Response
    where W: Write + Clone + Sync + Send + 'static
{
    let address_or_none = slice_as_array!(&address_vec, [u8; HASH_LEN]);
    if address_or_none.is_none() {
        return Response::Failed;
    }

    let chain = connection.handler().chain();
    let address = address_or_none.unwrap();

    // FIXME: Extremely slow, need to use metadata to 
    //        optimise this!
    let mut transactions = Vec::<(TransactionVariant, Option<Block>)>::new();
    chain.walk(&mut |block|
    {
        for transfer in &block.transfers 
        {
            if &transfer.header.to == address ||
                &transfer.get_from_address() == address
            {
                transactions.push((
                    TransactionVariant::Transfer(transfer.clone()), 
                    Some(block.clone())));
            }
        }

        for page in &block.pages
        {
            if &page.get_from_address() == address
            {
                transactions.push((
                    TransactionVariant::Page(page.clone()), 
                    Some(block.clone())));
            }
        }
    });

    // TODO: Pending
    Response::TransactionHistory(transactions)
}
