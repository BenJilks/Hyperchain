use crate::node::network::NetworkConnection;
use crate::node::Node;

use libhyperchain::service::command::Response;
use libhyperchain::block::Block;
use std::io::Write;

pub fn blocks<W>(connection: &mut NetworkConnection<Node<W>, W>, 
                 from: u64, to: u64) -> Response
    where W: Write + Clone + Send + Sync + 'static
{
    let chain = connection.handler().chain();
    let mut blocks = Vec::<Block>::new();
    for block_id in from..=to
    {
        match chain.block(block_id)
        {
            Some(block) => blocks.push(block),
            None => return Response::Failed,
        }
    }

    Response::Blocks(blocks)
}
