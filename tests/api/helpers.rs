use std::rc::Rc;
use std::{thread, time};

use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use argon2::{Algorithm, Params, Version};
use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;

use zero2prod::configuration::{get_configuration, DatabaseSettings};
use zero2prod::startup::Application;
use zero2prod::telemetry;

use crate::docker::{start_container, stop_container, Container};

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

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub db_pool: PgPool,
    pub email_server: MockServer,
    pub container_id: String,

    pub test_user: TestUser,
    pub api_client: reqwest::Client,
}

/// Confirmation links embedded in the request to the email API.
#[derive(Debug)]
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn post_newsletters(&self, body: serde_json::Value) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/newsletters", &self.address))
            .basic_auth(&self.test_user.username, Some(&self.test_user.password))
            .json(&body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    /// Extract the confirmation links embedded in the request to the email API.
    pub fn get_confirmation_links(&self, email_request: &wiremock::Request) -> ConfirmationLinks {
        let body: serde_json::Value = serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(&body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(&body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_login<Body>(&self, body: &Body) -> reqwest::Response
    where
        Body: serde::Serialize,
    {
        self.api_client
            .post(&format!("{}/login", &self.address))
            .form(body)
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_login_html(&self) -> String {
        self.api_client
            .get(&format!("{}/login", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
            .text()
            .await
            .unwrap()
    }

    pub async fn get_admin_dashborad(&self) -> reqwest::Response {
        self.api_client
            .get(&format!("{}/admin/dashboard", &self.address))
            .send()
            .await
            .expect("Failed to execute request.")
    }

    pub async fn get_admin_dashborad_html(&self) -> String {
        self.get_admin_dashborad().await.text().await.unwrap()
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        stop_container(self.container_id.clone()).expect("Failed to stop Postgres container");
    }
}

#[derive(Debug)]
pub struct TestUser {
    pub user_id: Uuid,
    pub username: String,
    pub password: String,
}

impl TestUser {
    pub fn generate() -> Self {
        Self {
            user_id: Uuid::new_v4(),
            username: Uuid::new_v4().to_string(),
            password: Uuid::new_v4().to_string(),
        }
    }

    async fn store(&self, pool: &PgPool) {
        let salt = SaltString::generate(&mut rand::thread_rng());
        // Match parameters of the default password
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(1500, 2, 1, None).unwrap(),
        )
        .hash_password(self.password.as_bytes(), &salt)
        .unwrap()
        .to_string();

        sqlx::query!(
            r#"
            INSERT INTO users (user_id, username, password_hash)
            VALUES ($1, $2, $3)
        "#,
            self.user_id,
            self.username,
            password_hash
        )
        .execute(pool)
        .await
        .expect("Failed to store test user.");
    }
}

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
pub async fn spawn_app() -> TestApp {
    // The first time `initialize` is invoked the code in `TRACING` is executed.
    // All other invocations will instead skip execution.
    Lazy::force(&TRACING);
    let db_name = Rc::new(Uuid::new_v4().to_string());

    // Launch a mock server to stand in for Postmark's API.
    let email_server = MockServer::start().await;

    // Randomise configuration to ensure test isolation
    let mut conf = {
        let mut c = get_configuration().expect("Failed to read configuration.");
        // Use a different database for each test case
        c.database.database_name = db_name.to_string();
        // Use a random OS port
        c.application.port = 0;

        // Use the mock server as email API
        c.email_client.base_url = email_server.uri();
        c
    };

    // Launch the application as a background task
    let (container, db_pool) = configure_database(&mut conf.database).await.unwrap();
    let app = Application::build(conf.clone())
        .await
        .expect("Failed to build application.");
    let application_port = app.port();
    let _ = tokio::spawn(app.run_until_stopped());

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .cookie_store(true)
        .build()
        .unwrap();
    let test_app = TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        db_pool,
        email_server,
        container_id: container.id,
        test_user: TestUser::generate(),
        api_client: client,
    };
    test_app.test_user.store(&test_app.db_pool).await;
    test_app
}

pub async fn configure_database(
    conf: &mut DatabaseSettings,
) -> Result<(Container, PgPool), anyhow::Error> {
    let image = "postgres:14-alpine".to_string();
    let port = "5432".to_string();
    let args: Vec<String> = vec![
        "-e".to_string(),
        "POSTGRES_USER=postgres".to_string(),
        "-e".to_string(),
        "POSTGRES_PASSWORD=password".to_string(),
    ];
    let container = start_container(image, port, args).expect("Failed to start Postgres container");
    // Create database
    conf.host = container.host.clone();
    conf.port = container.port;
    for i in 1..=10 {
        match PgConnection::connect_with(&conf.without_db()).await {
            Ok(conn) => {
                conn.close().await?;
                println!("Postgres are ready to go");
                break;
            }
            Err(err) => {
                if i == 10 {
                    return Err(anyhow::anyhow!(err));
                }
                println!("Postgres is not ready");
                let ten_millis = time::Duration::from_secs(i);
                thread::sleep(ten_millis);
            }
        }
    }

    let mut conn = PgConnection::connect_with(&conf.without_db())
        .await
        .expect("Cannot connect to Postgres");

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
        .expect("Failed to connect to Postgres with db");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    Ok((container, db_pool))
}

// #[tokio::test]
// async fn test_connect_db() {
//     // let mut ds = DatabaseSettings {
//     //     username: "postgres".to_string(),
//     //     password: secrecy::Secret::new("password".to_string()),
//     //     port: 5432,
//     //     host: "0.0.0.0".to_string(),
//     //     database_name: "test_database".to_string(),
//     //     require_ssl: false,
//     // };
//     // configure_database(&mut ds).await;

//     let app = spawn_app().await;

//     let resp = app.post_subscriptions("3424234".into()).await;
//     dbg!(resp);
// }

// Little helper function - we will be doing this check several times throughout
// this chapter and the next one.
pub fn assert_is_redirect_to(response: &reqwest::Response, location: &str) {
    assert_eq!(response.status().as_u16(), 303);
    assert_eq!(response.headers().get("Location").unwrap(), location);
}
