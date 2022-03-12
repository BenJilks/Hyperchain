/*
 * Copyright (c) 2022, Ben Jilks <benjyjilks@gmail.com>
 *
 * SPDX-License-Identifier: BSD-2-Clause
 */

use super::AppData;

use libhyperchain::service::client::Client;
use libhyperchain::service::command::{Command, Response};
use libhyperchain::transaction::Transaction;
use libhyperchain::transaction::page::Page;
use libhyperchain::data_store::data_unit::DataUnit;
use actix_web::{get, web};
use actix_web::{HttpRequest, HttpResponse, Responder};
use std::error::Error;

fn apply_data_unit(result: &mut Vec<u8>, site: &str, page_name: &str, data: DataUnit)
{
    match data
    {
        DataUnit::CreatePage(create_page) =>
        {
            info!("[{}] [{}] new page of {} bytes", site, page_name, create_page.page.len());
            if create_page.name == page_name {
                *result = create_page.page;
            }
        },
    }
}

fn render_updates(client: &mut Client, site: &str, page_name: &str, updates: &[Transaction<Page>]) 
    -> Result<Vec<u8>, Box<dyn Error>>
{
    let mut result = Vec::<u8>::new();
    for update in updates
    {
        let id = update.hash()?;
        match client.send(Command::PageData(id.data().to_vec()))?
        {
            Response::PageData(data) => 
                apply_data_unit(&mut result, site, page_name, data),

            _ => 
            {
                warn!("[{}] [{}] No data found for update '{}'",
                      site, page_name, id);
            },
        }
    }

    if result.is_empty() {
        Ok(format!("Page '{}' not found", page_name).as_bytes().to_vec())
    } else {
        Ok(result)
    }
}

fn get_page(client: &mut Client, site: String, page: String) -> impl Responder
{
    info!("[{}] Rendering new page {}", site, page);

    let id_or_error = base_62::decode(&site);
    if id_or_error.is_err() {
        return HttpResponse::Ok().body(format!("Unkown site: {}", site));
    }

    let id = id_or_error.unwrap();
    let response = client.send(Command::PageUpdates(id)).unwrap();
    match response
    {
        Response::PageUpdates(updates) =>
        {
            info!("[{}] [{}] Got {} updates", site, page, updates.len());
            let page = render_updates(client, &site, &page, &updates).unwrap();

            HttpResponse::Ok()
                .header("Location", format!("/site/{}/", site))
                .body(page)
        },

        _ => HttpResponse::Ok().body(format!("Unkown site: {}", site)),
    }
}

#[get("/site/{site}")]
pub async fn site_index_redirect_handler(web::Path(site): web::Path<String>)
    -> impl Responder
{
    HttpResponse::PermanentRedirect()
        .header("Location", format!("/site/{}/", site))
        .body("")
}

#[get("/site/{site}/")]
pub async fn site_index_handler(web::Path(site): web::Path<String>,
                                request: HttpRequest)
    -> impl Responder
{
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();
    let mut client = app_data.client();
    get_page(&mut client, site, "index.html".to_owned())
}

#[get("/site/{site}/{page:.*}")]
pub async fn site_handler(web::Path((site, page)): web::Path<(String, String)>, 
                          request: HttpRequest) 
    -> impl Responder
{
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();
    let mut client = app_data.client();
    get_page(&mut client, site, page)
}

