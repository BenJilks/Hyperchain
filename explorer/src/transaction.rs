use crate::AppData;

use libhyperchain::transaction::Transaction;
use libhyperchain::transaction::transfer::Transfer;
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

pub fn data_for_transaction((transaction, block): &(Transaction<Transfer>, Option<Block>)) 
    -> serde_json::Value
{
    let hash = transaction.hash().unwrap();
    let id = base_62::encode(&hash);
    let from = base_62::encode(&transaction.get_from_address());
    let to = base_62::encode(&transaction.header.to);

    let block_id = 
        match block
        {
            Some(block) => block.block_id.to_string(),
            None => "Pending".to_owned(),
        };

    json!({
        "id": id,
        "from": from,
        "to": to,
        "amount": transaction.header.amount,
        "fee": transaction.header.fee,
        "block": block_id,
    })
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
