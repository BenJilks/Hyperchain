use super::AppData;

use libhyperchain::service::command::{Command, Response};
use actix_web::{get, web};
use actix_web::{HttpRequest, HttpResponse, Responder};

#[get("/")]
pub async fn index_handler(request: HttpRequest) 
    -> impl Responder
{
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();
    let mut client = app_data.client();
    match client.send(Command::Statistics).unwrap()
    {
        Response::Statistics(stats) =>
        {
            let data = json!(
            {
                "hash_rate": stats.hash_rate,
                "known_chunks": stats.known_chunks,
                "replication_percent": stats.replication * 100.0,
            });

            let body = app_data.hb.render("index", &data).unwrap();
            HttpResponse::Ok().body(body)
        },

        _ =>  HttpResponse::Ok().body("Error: Unable to fetch statistics"),
    }
}

