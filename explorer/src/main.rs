extern crate actix_web;
extern crate actix_files;
extern crate handlebars;
extern crate serde;
extern crate base_62;

#[macro_use]
extern crate serde_json;

use libhyperchain::service::client::Client;
use libhyperchain::service::command::{Command, Response};
use libhyperchain::wallet::WalletStatus;
use actix_web::{get, web, App};
use actix_web::{HttpServer, HttpRequest, HttpResponse, Responder};
use actix_files::Files;
use handlebars::Handlebars;
use serde::Deserialize;
use std::error::Error;
use std::sync::{Arc, Mutex, MutexGuard};

struct AppData<'a>
{
    client: Arc<Mutex<Client>>,
    hb: Handlebars<'a>,
}

impl<'a> AppData<'a>
{

    pub fn client(&self) -> MutexGuard<Client>
    {
        self.client.lock().unwrap()
    }

}

#[derive(Deserialize)]
struct WalletParameters
{
    address: String,
}

fn get_wallet_status(client: &mut Client, address: &str) -> WalletStatus
{
    let address = base_62::decode(address).unwrap();
    let wallet_status = client.send(Command::Balance(address)).unwrap();
    match wallet_status
    {
        Response::WalletStatus(status) => status,
        _ => WalletStatus::default(),
    }
}

#[get("/wallet")]
async fn wallet(request: HttpRequest) -> impl Responder
{
    let parameters = web::Query::<WalletParameters>::from_query(request.query_string()).unwrap();
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();

    let mut client = app_data.client();
    let wallet_status = get_wallet_status(&mut client, &parameters.address);

    let data = json!({
        "address": parameters.address,
        "balance": wallet_status.balance,
    });
    
    let body = app_data.hb.render("wallet", &data).unwrap();
    HttpResponse::Ok().body(body)
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>>
{
    let mut handlebars = Handlebars::new();
    handlebars.register_templates_directory(".html", "./static/templates")?;
    
    let app_data = web::Data::new(AppData
    {
        client: Arc::from(Mutex::from(Client::new()?)),
        hb: handlebars,
    });

    HttpServer::new(move || 
        {
            App::new()
                .app_data(app_data.clone())
                .service(wallet)
                .service(Files::new("/", "./static/root").index_file("index.html"))
        })
        .bind("127.0.0.1:8080")?
        .run()
        .await?;

    Ok(())
}
