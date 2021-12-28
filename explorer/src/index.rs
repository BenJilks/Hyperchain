use super::AppData;

use actix_web::{get, web};
use actix_web::{HttpRequest, HttpResponse, Responder};

#[get("/")]
pub async fn index_handler(request: HttpRequest) 
    -> impl Responder
{
    let app_data = request.app_data::<web::Data<AppData>>().unwrap();
    // let mut client = app_data.client();

    let data = json!(
    {
    });

    let body = app_data.hb.render("index", &data).unwrap();
    HttpResponse::Ok().body(body)
}

