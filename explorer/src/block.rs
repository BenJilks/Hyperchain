use super::AppData;
use super::transaction::data_for_transaction;

use libhyperchain::service::command::{Command, Response};
use libhyperchain::service::client::Client;
use libhyperchain::block::target::difficulty;
use actix_web::{get, web};
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::Deserialize;
use std::error::Error;

#[derive(Deserialize)]
struct BlockParameters
{
    id: String,
}

fn get_top_block_id(client: &mut Client) -> Result<u64, Box<dyn Error>>
{
    match client.send(Command::TopBlock)?
    {
        Response::Blocks(blocks) if (blocks.len() == 1) =>
            Ok(blocks[0].header.block_id),

        _ => Ok(0),
    }
}

#[get("/block")]
pub async fn block_handler(request: HttpRequest) -> impl Responder
{
    let parameters = web::Query::<BlockParameters>::from_query(request.query_string()).unwrap();
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();

    let mut client = app_data.client();
    let block_id = parameters.id.parse::<u64>().unwrap();
    let top_block_id = get_top_block_id(&mut client).unwrap();
    match client.send(Command::Blocks(block_id, block_id)).unwrap()
    {
        Response::Blocks(blocks) if (blocks.len() == 1) =>
        {
            let block = &blocks[0];
            let winner = base_62::encode(&block.header.raward_to);
            let difficulty = difficulty(&block.header.target);

            let data = json!({
                "id": block_id,
                "next_block_id": block_id + 1,
                "last_block_id": block_id - 1,
                "top_block_id": top_block_id,
                "timestamp": (block.header.timestamp / 1000) as u64,
                "winner": winner,
                "merkle_root": base_62::encode(&block.header.transaction_merkle_root),
                "difficulty": difficulty,
                "pow": block.header.pow,
                "transactions":
                    block.transactions()
                        .iter()
                        .map(|x| (x.clone(), Some(block.clone())))
                        .collect::<Vec<_>>()
                        .iter()
                        .map(data_for_transaction)
                        .collect::<Vec<_>>(),
            });
        
            let body = app_data.hb.render("block", &data).unwrap();
            HttpResponse::Ok().body(body)
        },

        _ => HttpResponse::Ok().body("Error: Block not found"),
    }
}

