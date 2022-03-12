/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::hash::Hash;

pub fn transaction_history(connection: &mut NetworkConnection<NodePacketHandler>,
                           address_vec: Vec<u8>) -> Response
{
    // TODO: Varify this is a valid hash
    let address = Hash::from(&address_vec);

    let mut node = connection.handler().node();
    let chain = node.chain();
    let transactions = chain.get_transaction_history(&address);
    Response::TransactionHistory(transactions)
}

