// `tokio::test` is the testing equivalent of `tokio::main`.
// It also spares you from having to specify the `#[test]` attribute.
//
// You can inspect what code gets generated using
// `cargo expand --test health_check` (<- name of the test file)

use std::net::TcpListener;

use reqwest;
use sqlx::PgPool;

use zero2prod::configuration::get_configuration;
use zero2prod::startup::run;

#[tokio::test]
async fn health_check_works() {
    // Arrange
    let app = spawn_app().await;

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
    assert_eq!(Some(0), response.content_length())
}

#[tokio::test]
async fn subscribe_returns_a_400_for_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;
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
            "The API did not fail with 400 Bad Request when the payload was {}.", error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;
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

    let saved = sqlx::query!("SELECT email, name FROM subscriptions").fetch_one(&app.db_pool).await.expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "benjamin@gmail.com");
    assert_eq!(saved.name, "benjamin")
}

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

/// Spin up an instance of our application
/// and returns its address (i.e. http://localhost:XXXX)
async fn spawn_app() -> TestApp {
    let lis = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    // We retrieve the port assigned to us by the OS
    let port = lis.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);
    let conf = get_configuration().expect("Failed to read configuration.");
    let db_pool = PgPool::connect(&conf.database.connection_string()).await.expect("Failed to connect to Postgres.");

    let server = run(lis, db_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp { address, db_pool }
}
