use crate::AppData;

use libhyperchain::transaction::{Transaction, TransactionContent, TransactionVariant};
use libhyperchain::transaction::transfer::Transfer;
use libhyperchain::transaction::page::Page;
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

fn transfer_data(transfer: &Transaction<Transfer>, block_id: String) 
    -> serde_json::Value
{
    let hash = transfer.hash().unwrap();
    let id = base_62::encode(&hash);

    let inputs = inputs_from_transaction(&transfer);
    let outputs = transfer.header.content.outputs
        .iter()
        .map(|output|
        {
            json!(
            {
                "address": base_62::encode(&output.to),
                "amount": output.amount,
            })
        })
        .collect::<Vec<_>>();

    let total_amount = transfer.header.content.outputs
        .iter()
        .fold(0.0, |acc, x| acc + x.amount);

    json!(
    {
        "type": "Transfer",
        "id": id,
        "inputs": inputs,
        "outputs": outputs,
        "total_amount": total_amount,
        "fee": transfer.header.content.fee,
        "block": block_id,
    })
}

fn page_data(page: &Transaction<Page>, block_id: String) 
    -> serde_json::Value
{
    let hash = page.hash().unwrap();
    let id = base_62::encode(&hash);

    let inputs = inputs_from_transaction(&page);
    let outputs = vec![json!(
    {
        "address": base_62::encode(&page.header.content.site),
        "amount": page.header.content.cost(),
    })];

    let data = page.header.content.data_hashes
        .iter()
        .map(|x| json!(
        {
            "hash": base_62::encode(x),
        }))
        .collect::<Vec<_>>();

    let data_size_bytes = page.header.content.data_length;
    let data_size = data_size_bytes as f32 / (1000.0 * 1000.0);
    let chunk_count = page.header.content.data_hashes.len();

    json!(
    {
        "type": "Page Update",
        "id": id,
        "inputs": inputs,
        "outputs": outputs,
        "data": data,
        "amount": page.header.content.cost(),
        "fee": page.header.content.fee,
        "block": block_id,
        "data_size": data_size,
        "chunk_count": chunk_count,
    })
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
        TransactionVariant::Transfer(transfer) => transfer_data(transfer, block_id),
        TransactionVariant::Page(page) => page_data(page, block_id),
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

