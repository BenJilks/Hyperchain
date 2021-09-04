use super::AppData;
use super::transaction::data_for_transaction;

use libhyperchain::service::command::{Command, Response};
use libhyperchain::block::Block;
use libhyperchain::block::target::difficulty;
use actix_web::{get, web};
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
struct BlockParameters
{
    id: String,
}

#[get("/block")]
pub async fn block_handler(request: HttpRequest) -> impl Responder
{
    let parameters = web::Query::<BlockParameters>::from_query(request.query_string()).unwrap();
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();

    let mut client = app_data.client();
    let block_id = parameters.id.parse::<u64>().unwrap();
    match client.send(Command::Blocks(block_id, block_id)).unwrap()
    {
        Response::Blocks(blocks) if (blocks.len() == 1) =>
        {
            let block = &blocks[0];
            let winner = base_62::encode(&block.raward_to);
            let difficulty = difficulty(&block.target);

            let data = json!({
                "id": block_id,
                "timestamp": (block.timestamp / 1000) as u64,
                "winner": winner,
                "difficulty": difficulty,
                "pow": block.pow,
                "transactions":
                    block.transfers
                        .iter()
                        .map(|x| (x.clone(), None::<Block>))
                        .collect::<Vec<_>>()
                        .iter()
                        .map(data_for_transaction)
                        .collect::<Vec<_>>()
            });
        
            let body = app_data.hb.render("block", &data).unwrap();
            HttpResponse::Ok().body(body)
        },

        _ => HttpResponse::Ok().body("Error: Block not found"),
    }
}
