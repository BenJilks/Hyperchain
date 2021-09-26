use crate::AppData;

use libhyperchain::transaction::{Transaction, TransactionContent, TransactionVariant};
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

fn inputs_from_transaction<C>(transaction: &Transaction<C>) 
    -> Vec<serde_json::Value>
    where C: TransactionContent
{
    transaction.header.inputs
        .iter()
        .map(|input| 
        {
            json!(
            {
                "address": base_62::encode(&input.get_address()),
                "amount": input.amount,
            })
        })
        .collect::<Vec<_>>()
}

pub fn data_for_transaction((transaction, block): &(TransactionVariant, Option<Block>)) 
    -> serde_json::Value
{
    let block_id = 
        match block
        {
            Some(block) => block.header.block_id.to_string(),
            None => "Pending".to_owned(),
        };

    match transaction
    {
        TransactionVariant::Transfer(transfer) =>
        {
            let hash = transfer.hash().unwrap();
            let id = base_62::encode(&hash);

            let inputs = inputs_from_transaction(&transfer);
            let outputs = vec![json!(
            {
                "address": base_62::encode(&transfer.header.content.to),
                "amount": transfer.header.content.amount,
            })];

            json!(
            {
                "type": "Transfer",
                "id": id,
                "inputs": inputs,
                "outputs": outputs,
                "total_amount": transfer.header.content.amount,
                "fee": transfer.header.content.fee,
                "block": block_id,
            })
        },

        TransactionVariant::Page(page) =>
        {
            let hash = page.hash().unwrap();
            let id = base_62::encode(&hash);

            let inputs = inputs_from_transaction(&page);
            let outputs = vec![json!(
            {
                "address": base_62::encode(&page.header.content.site),
                "amount": page.header.content.cost(),
            })];
        
            json!(
            {
                "type": "Page Update",
                "id": id,
                "inputs": inputs,
                "outputs": outputs,
                "amount": page.header.content.cost(),
                "fee": page.header.content.fee,
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

