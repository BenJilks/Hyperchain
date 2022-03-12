/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::network::NetworkConnection;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::transaction::TransactionVariant;
use libhyperchain::hash::Hash;

pub fn page_updates(connection: &mut NetworkConnection<NodePacketHandler>,
                    address_vec: Vec<u8>) -> Response
{
    info!("Requested page updates for address '{}'", base_62::encode(&address_vec));

    let mut node = connection.handler().node();
    let chain = node.chain();

    // TODO: Varify this is a valid hash
    let address = Hash::from(&address_vec);
    Response::PageUpdates(chain.get_page_updates(&address))
}

pub fn page_data(connection: &mut NetworkConnection<NodePacketHandler>, 
                 transaction_id: Vec<u8>) -> Response
{
    let mut node = connection.handler().node();
    let chain = node.chain();

    let transaction_id_hash = Hash::from(&transaction_id);
    let transaction_or_none = chain.find_transaction(&transaction_id_hash);
    if transaction_or_none.is_none() {
        return Response::Failed;
    }

    let (transaction, _) = transaction_or_none.unwrap();
    match transaction
    {
        TransactionVariant::Page(page) =>
        {
            info!("Fetching page data for transaction '{}'", base_62::encode(&transaction_id));
            match node.data_store().get_data_unit(&page)
            {
                Ok(data) => Response::PageData(data),
                Err(_) => Response::Failed,
            }
        },

        _ => Response::Failed,
    }

}

