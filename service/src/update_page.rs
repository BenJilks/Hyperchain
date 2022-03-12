/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use crate::network::NetworkConnection;
use crate::network::packet::Packet;
use crate::node::packet_handler::NodePacketHandler;

use libhyperchain::service::command::Response;
use libhyperchain::wallet::private_wallet::PrivateWallet;
use libhyperchain::transaction::Transaction;
use libhyperchain::transaction::page::Page;
use libhyperchain::data_store::data_unit::DataUnit;
use libhyperchain::data_store::page::CreatePageData;

fn add_page(connection: &mut NetworkConnection<NodePacketHandler>,
            from: Vec<u8>, data_unit: &DataUnit)
    -> Option<(Transaction<Page>, Vec<u8>)>
{
    let from_wallet_or_error = PrivateWallet::deserialize(from);
    if from_wallet_or_error.is_err() {
        return None;
    }
    
    let mut node = connection.handler().node();
    let chain = &mut node.chain();
    let from_wallet = from_wallet_or_error.unwrap();
    let page_or_error = chain.new_page(&from_wallet, &data_unit, 1.0);
    if page_or_error.is_err() 
    {
        warn!("Error in send: {}", page_or_error.unwrap_err());
        return None;
    }

    let page = page_or_error.unwrap();
    let result = chain.push_page_queue(page.clone());
    if result.is_err()
    {
        warn!("Error in send: {}", result.unwrap_err());
        return None;
    }
    
    let page_id = page.hash().unwrap();
    Some((page, page_id.data().to_vec()))
}

pub fn update_page(connection: &mut NetworkConnection<NodePacketHandler>,
                   from: Vec<u8>, name: String, data: Vec<u8>)
    -> Response
{
    let data_unit = DataUnit::CreatePage(CreatePageData::new(name, data));
    let page_or_none = add_page(connection, from, &data_unit);
    if page_or_none.is_none() {
        return Response::Failed;
    }

    // TODO: Handle errors.

    let (page, page_id) = page_or_none.unwrap();
    connection.handler().node().data_store().store_data_unit(&data_unit).unwrap();
    connection.manager().send(Packet::Page(page, data_unit)).unwrap();

    let report = connection.handler().node().our_report().unwrap();
    connection.manager().send(Packet::Report(None, report)).unwrap();

    Response::Sent(page_id)
}

