use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;

use zero2prod::configuration::{get_configuration};
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
use zero2prod::telemetry;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber("zero2prod".into(), "info".into(), std::io::stdout);
    telemetry::init_subscriber(subscriber);

    // Panic if we can't read configuration
    let conf = get_configuration().expect("Failed to read configuration.");
    let db_pool = PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_secs(2))
            .connect_lazy_with(conf.database.with_db());
        
    let sender_email = conf.email_client.sender().expect("Invalid sender email address.");
    let email_client = EmailClient::new(conf.email_client.base_url, sender_email, conf.email_client.authorization_token);
    let addr = format!("{}:{}", conf.application.host, conf.application.port);
    let lis = TcpListener::bind(addr)?;
    run(lis, db_pool, email_client)?.await?;
    Ok(())
}
