use std::net::TcpListener;

use sqlx::PgPool;

use zero2prod::startup::run;
use zero2prod::configuration::get_configuration;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Panic if we can't read configuration
    let conf = get_configuration().expect("Failed to read configuration.");
    let db_pool = PgPool::connect(&conf.database.connection_string()).await.expect("Failed to connect to Postgres.");
    // We have removed the hard-coded `18000` - it's now coming from our settings!
    let addr = format!("127.0.0.1:{}", conf.application_port);
    let lis = TcpListener::bind(addr)?;
    run(lis, db_pool)?.await
}
