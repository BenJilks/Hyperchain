extern crate actix_web;
extern crate actix_files;
extern crate handlebars;
extern crate serde;
extern crate base_62;
extern crate pretty_env_logger;

#[macro_use]
extern crate serde_json;

mod wallet;
mod transaction;
mod block;
mod site;

use libhyperchain::service::client::Client;
use actix_web::{web, App};
use actix_web::HttpServer;
use actix_files::Files;
use handlebars::Handlebars;
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

#[actix_web::main]
async fn main() -> Result<(), Box<dyn Error>>
{
    pretty_env_logger::init();
    
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
                .service(wallet::wallet_handler)
                .service(transaction::transaction_handler)
                .service(block::block_handler)
                .service(site::site_index_redirect_handler)
                .service(site::site_index_handler)
                .service(site::site_handler)
                .service(Files::new("/", "./static/root").index_file("index.html"))
        })
        .bind("0.0.0.0:8080")?
        .run()
        .await?;

    Ok(())
}
