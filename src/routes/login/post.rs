use actix_web::http::header::LOCATION;
use actix_web::{web, HttpResponse};
use secrecy::Secret;

#[derive(Debug, serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

pub async fn login(_form: web::Data<FormData>) -> HttpResponse {
    HttpResponse::Ok().insert_header((LOCATION, "/")).finish()
}
