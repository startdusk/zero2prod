use actix_web::http::header::LOCATION;
use actix_web::HttpResponse;

pub async fn login() -> HttpResponse {
    HttpResponse::Ok().insert_header((LOCATION, "/")).finish()
}
