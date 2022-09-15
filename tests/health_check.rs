// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)

use std::net::TcpListener;
use tokio;

use reqwest;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

use once_cell::sync::Lazy;
use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::email_client::EmailClient;
use zero2prod::startup::run;
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

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let mut app = spawn_app().await;

    // We need to bring in `reqwest`
    // to perform HTTP requests against our appllication.
    let client = reqwest::Client::new();

    // Act
    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    app.drop_database().await
}

#[tokio::test]
async fn subscribe_returns_a_400_for_when_data_is_missing() {
    // Arrange
    let mut app = spawn_app().await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=benjamin", "missing the email"),
        ("email=benjamin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }

    app.drop_database().await
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let mut app = spawn_app().await;
    let client = reqwest::Client::new();

    // Act
    let body = "name=benjamin&email=benjamin%40gmail.com";
    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    // Assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "benjamin@gmail.com");
    assert_eq!(saved.name, "benjamin");

    app.drop_database().await
}

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_invalid() {
    // Arrange 
    let mut app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=&email=benjamin%40gmail.com", "empty name"),
        ("name=benjamin&email=", "empty email"),
        ("name=benjamin&email=definitely-not-an-email", "invalid email"),
    ];

    for (body, description) in test_cases {
        // Act
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.");

        // Assert
        assert_eq!(400, response.status().as_u16(), "The API did not return a 200 OK when the payload was {}.", description);
    }

    app.drop_database().await
}

#[derive(Debug)]
pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
    pub db_name: String,
    pub pg_conn: PgConnection,
}

impl TestApp {
    pub async fn drop_database(&mut self) {
        self.db_pool.close().await;
        self.pg_conn.execute(
            format!(
                r#"
                DROP DATABASE "{}";
            "#,
                self.db_name
            )
            .as_str(),
        )
        .await
        .expect(&format!("Failed to drop database: {}", self.db_name));
    }
}

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);

    let lis = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = lis.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let mut conf = get_configuration().expect("Failed to read configuration.");
    conf.database.database_name = Uuid::new_v4().to_string();
    let database = conf.database;
    let (pg_conn, db_pool) = configure_database(&database).await;
    // Build a new email client
    let sender_email = conf.email_client.sender().expect("Invalid sender email address.");
    let email_client = EmailClient::new(
        conf.email_client.base_url,
        sender_email,
        conf.email_client.authorization_token
    );
    let server = run(lis, db_pool.clone(), email_client).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    
    TestApp { address, db_pool, db_name: database.database_name, pg_conn }
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
