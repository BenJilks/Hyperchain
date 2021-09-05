use crate::AppData;

use libhyperchain::transaction::TransactionVariant;
use libhyperchain::block::Block;
use libhyperchain::service::command::{Command, Response};
use actix_web::{get, web};
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
struct TransactionParameters
{
    id: String,
}

pub fn data_for_transaction((transaction, block): &(TransactionVariant, Option<Block>)) 
    -> serde_json::Value
{
    let block_id = 
        match block
        {
            Some(block) => block.block_id.to_string(),
            None => "Pending".to_owned(),
        };

    match transaction
    {
        TransactionVariant::Transfer(transfer) =>
        {
            let hash = transfer.hash().unwrap();
            let id = base_62::encode(&hash);
            let from = base_62::encode(&transfer.get_from_address());
            let to = base_62::encode(&transfer.header.to);
        
            json!({
                "type": "Transfer",
                "id": id,
                "from": from,
                "to": to,
                "amount": transfer.header.amount,
                "fee": transfer.header.fee,
                "block": block_id,
            })
        },

        TransactionVariant::Page(page) =>
        {
            let hash = page.hash().unwrap();
            let id = base_62::encode(&hash);
            let from = base_62::encode(&page.get_from_address());
        
            json!({
                "type": "Page Update",
                "id": id,
                "from": from,
                "amount": page.header.cost(),
                "fee": page.header.fee,
                "block": block_id,
            })
        },
    }
}

#[get("/transaction")]
pub async fn transaction_handler(request: HttpRequest) -> impl Responder
{
    let parameters = web::Query::<TransactionParameters>::from_query(request.query_string()).unwrap();
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();

    let mut client = app_data.client();
    let id = base_62::decode(&parameters.id).unwrap();
    match client.send(Command::TransactionInfo(id)).unwrap()
    {
        Response::TransactionInfo(transaction, block) =>
        {
            let body = app_data.hb.render("transaction", 
                &data_for_transaction(&(transaction, block))).unwrap();
            HttpResponse::Ok().body(body)
        },

        _ => HttpResponse::Ok().body("Transaction not found"),
    }
}
