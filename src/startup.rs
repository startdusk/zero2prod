use std::net::TcpListener;

use actix_web::{HttpServer, App, web};
use actix_web::dev::Server;
use actix_web::web::Data;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tracing_actix_web::TracingLogger;

use crate::configuration::{Settings, DatabaseSettings};
use crate::email_client::EmailClient;
use crate::routes::{health_check, subscribe};

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
        let server = run(lis, conn_pool, email_client)?;
        
        Ok(Self{ port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

pub fn get_connection_pool(conf: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new().acquire_timeout(std::time::Duration::from_secs(2)).connect_lazy_with(conf.with_db())
}

pub fn run(lis: TcpListener, conn_pool: PgPool, email_client: EmailClient) -> Result<Server, std::io::Error> {
    // Wrap the pool using web::Data, which boils down to an Arc smart pointer
    let conn_pool = Data::new(conn_pool);
    let email_client = Data::new(email_client);
    let srv = HttpServer::new(move || {
        App::new()
            // Middleware are added using the `wrap` method on `App`
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            // A new entry in our routing table for POST /subscriptions requests
            .route("/subscriptions", web::post().to(subscribe))
            // Register the connection as part of the application state
            .app_data(conn_pool.clone())
            .app_data(email_client.clone())
    })
    .listen(lis)?
    .run();

    Ok(srv)
}

