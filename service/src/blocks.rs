/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::block::Block;

pub fn blocks(connection: &mut NetworkConnection<NodePacketHandler>,
              from: u64, to: u64) 
    -> Response
{
    let mut node = connection.handler().node();
    let chain = node.chain();

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

pub fn top_block(connection: &mut NetworkConnection<NodePacketHandler>)
    -> Response
{
    let mut node = connection.handler().node();
    let chain = node.chain();

    match chain.top()
    {
        Some(top) => Response::Blocks(vec![top]),
        None => Response::Failed,
    }
}

