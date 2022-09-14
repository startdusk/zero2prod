use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;
use zero2prod::configuration::get_configuration;
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
        
    let addr = format!("{}:{}", conf.application.host, conf.application.port);
    let lis = TcpListener::bind(addr)?;
    run(lis, db_pool)?.await?;
    Ok(())
}
