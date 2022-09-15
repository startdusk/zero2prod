use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::{web::{self, Data}, App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::routes::{health_check, subscribe};
use crate::email_client::{EmailClient};

pub fn run(lis: TcpListener, db_pool: PgPool, email_client: EmailClient) -> Result<Server, std::io::Error> {
    // Wrap the pool using web::Data, which boils down to an Arc smart pointer
    let db_pool = Data::new(db_pool);
    let email_client = Data::new(email_client);
    let srv = HttpServer::new(move || {
        App::new()
            // Middleware are added using the `wrap` method on `App`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            // A new entry in our routing table for POST /subscriptions requests
            .route("/subscriptions", web::post().to(subscribe))
            // Register the connection as part of the application state
            .app_data(db_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(lis)?
    .run();

    Ok(srv)
}

