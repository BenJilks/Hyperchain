use super::AppData;
use super::transaction::data_for_transaction;

use libhyperchain::service::command::{Command, Response};
use libhyperchain::wallet::WalletStatus;
use libhyperchain::transaction::Transaction;
use libhyperchain::block::Block;
use libhyperchain::service::client::Client;
use actix_web::{get, web};
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
struct WalletParameters
{
    address: String,
}

fn get_wallet_status(client: &mut Client, address: &Vec<u8>) -> WalletStatus
{
    let wallet_status = client.send(Command::Balance(address.clone())).unwrap();
    match wallet_status
    {
        Response::WalletStatus(status) => status,
        _ => WalletStatus::default(),
    }
}

fn get_transaction_history(client: &mut Client, address: &Vec<u8>) 
    -> Vec<(Transaction, Option<Block>)>
{
    let wallet_status = client.send(Command::TransactionHistory(address.clone())).unwrap();
    match wallet_status
    {
        Response::TransactionHistory(history) => history,
        _ => Vec::new(),
    }
}

#[get("/wallet")]
pub async fn wallet_handler(request: HttpRequest) -> impl Responder
{
    let parameters = web::Query::<WalletParameters>::from_query(request.query_string()).unwrap();
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();

    let mut client = app_data.client();
    let address = base_62::decode(&parameters.address).unwrap();
    let wallet_status = get_wallet_status(&mut client, &address);
    let transacion_history = get_transaction_history(&mut client, &address);

    let data = json!({
        "address": parameters.address,
        "balance": wallet_status.balance,
        "transaction_count": transacion_history.len(),
        "history": 
            transacion_history
                .iter()
                .map(data_for_transaction)
                .collect::<Vec<_>>(),
    });
    
    let body = app_data.hb.render("wallet", &data).unwrap();
    HttpResponse::Ok().body(body)
}
