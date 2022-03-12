use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::{Response, Statistics};
use libhyperchain::block::target::{difficulty, hash_rate, calculate_target};

pub fn statistics(connection: &mut NetworkConnection<NodePacketHandler>)
    -> Response
{
    let mut node = connection.handler().node();

    let (sample_start, sample_end) = node.chain().take_sample();
    let hash_rate = hash_rate(difficulty(&calculate_target(sample_start, sample_end)), 1);

    // TODO: Handle errors.
    let usage = node.storage_usage().unwrap();

    let total_chunks = usage.len();
    let total_chunks_stored = usage
        .iter()
        .fold(0, |acc, (_, count)| acc + count);

    let replication =
        if total_chunks > 0 {
            total_chunks_stored as f64 / total_chunks as f64
        } else {
            1.0
        };

    Response::Statistics(Statistics
    {
        hash_rate,
        known_chunks: total_chunks,
        replication,
    })
}

