use std::rc::Rc;
use std::thread;

use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::migrate::MigrateDatabase;
use sqlx::{Connection, Executor, PgConnection, PgPool, Postgres};
use tokio::runtime::Runtime;
use uuid::Uuid;

use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::Application;
use zero2prod::telemetry;

// Ensure that the `tracing` stack is only initialised once using `once_cell`
static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    // We cannot assign the output of `get_subscriber` to a variable based on the value of `TEST_LOG`
    // because the sink is part of the type returned by `get_subscriber`, therefore they are not the
    // same type. We could work around it, but this is the most straight-forward way of moving forward.
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::stdout);
        telemetry::init_subscriber(subscriber);
    } else {
        let subscriber =
            telemetry::get_subscriber(subscriber_name, default_filter_level, std::io::sink);
        telemetry::init_subscriber(subscriber);
    };
});

#[derive(Debug)]
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_url: String,
    pub pg_conn: PgConnection,
}

impl TestApp {
    // pub async fn drop_database(&mut self) {
    //     if !self.db_pool.is_closed() {
    //         println!("db_pool not closed ");
    //         self.db_pool.close().await;
    //     }
    //     self.pg_conn
    //         .execute(
    //             format!(
    //                 r#"
    //             DROP DATABASE "{}";
    //         "#,
    //                 self.db_name
    //             )
    //             .as_str(),
    //         )
    //         .await
    //         .expect(&format!("Failed to drop database: {}", self.db_name));
    // }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        reqwest::Client::new()
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let pool = self.db_pool.to_owned();
        let db_url = self.db_url.clone();
        thread::spawn(move || {
            let runtime = Runtime::new().unwrap();
            runtime.block_on(async {
                println!("inside runtime");
                pool.close().await;
                println!("closed");
                drop(pool);
                println!("dropped");
                Postgres::drop_database(&db_url).await.unwrap();
                println!("dropped db");
            });
        })
        .join()
        .expect("thread failed");
    }
}

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);
    let db_name = Rc::new(Uuid::new_v4().to_string());
    // Randomise configuration to ensure test isolation
    let conf = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = db_name.to_string();
        // Use a random OS port
        c.application.port = 0;
        c
    };

    // Launch the application as a background task
    let (pg_conn, db_pool) = configure_database(&conf.database).await;
    let app = Application::build(conf.clone())
        .await
        .expect("Failed to build application.");
    let application_port = app.port();
    let _ = tokio::spawn(app.run_until_stopped());

    TestApp {
        address: format!("http://localhost:{}", application_port),
        db_pool,
        db_url: format!(
            "postgres://{}:{}@{}:{}/{}",
            conf.database.username,
            conf.database.password.expose_secret(),
            conf.database.host,
            conf.database.port,
            db_name.to_string()
        ),
        pg_conn,
    }
}

pub async fn configure_database(conf: &DatabaseSettings) -> (PgConnection, PgPool) {
    // Create database
    let mut conn = PgConnection::connect_with(&conf.without_db())
        .await
        .expect("Failed to connect to Postgres");

    conn.execute(
        format!(
            r#"
            CREATE DATABASE "{}";
        "#,
            conf.database_name
        )
        .as_str(),
    )
    .await
    .expect("Failed to create database");

    // Migrate database
    let db_pool = PgPool::connect_with(conf.with_db())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    (conn, db_pool)
}
