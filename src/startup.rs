use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::configuration::{DatabaseSettings, Settings};
use crate::email_client::EmailClient;
use crate::routes::{
    confirm, health_check, home, login, login_form, publish_newsletter, subscribe,
};

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    pub async fn build(conf: Settings) -> Result<Self, std::io::Error> {
        let conn_pool = get_connection_pool(&conf.database);
        let email_client = conf.email_client.client();

        let addr = format!("{}:{}", conf.application.host, conf.application.port);
        let lis = TcpListener::bind(addr)?;
        let port = lis.local_addr().unwrap().port();
        let server = run(lis, conn_pool, email_client, conf.application.base_url)?;

        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

// We need to define a wrapper type in order to retrieve the URL
// in the `subscribe` handler.
// Retrieval from the context, in actix-web, is type-based: using
// a raw `String` would expose us to conflicts.
#[derive(Debug)]
pub struct ApplicationBaseUrl(pub String);

pub fn get_connection_pool(conf: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(conf.with_db())
}

pub fn run(
    lis: TcpListener,
    conn_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    // Wrap the pool using web::Data, which boils down to an Arc smart pointer
    let conn_pool = Data::new(conn_pool);
    let email_client = Data::new(email_client);
    let base_url = Data::new(ApplicationBaseUrl(base_url));
    let srv = HttpServer::new(move || {
        App::new()
            // Middleware are added using the `wrap` method on `App`
            .wrap(TracingLogger::default())
            .route("/", web::get().to(home))
            .route("/login", web::get().to(login_form))
            .route("/login", web::post().to(login))
            .route("/health_check", web::get().to(health_check))
            // A new entry in our routing table for POST /subscriptions requests
            .route("/subscriptions", web::post().to(subscribe))
            // Register the connection as part of the application state
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .app_data(conn_pool.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(lis)?
    .run();

    Ok(srv)
}
