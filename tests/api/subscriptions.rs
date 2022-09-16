use crate::helpers::spawn_app;


#[tokio::test]
async fn subscribe_returns_a_400_for_when_data_is_missing() {
    // Arrange
    let app = spawn_app().await;

    let test_cases = vec![
        ("name=benjamin", "missing the email"),
        ("email=benjamin%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        // Act
        let response = app.post_subscriptions(invalid_body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            // Additional customised error message on test failure
            "The API did not fail with 400 Bad Request when the payload was {}.",
            error_message
        );
    }
}

#[derive(Debug)]
struct SavedData {
    name: String,
    email: String,
    status: String,
}

#[ignore = "sqlx 0.6.0: Pool::close does not completely close when awaited #1928"]
#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let body = "name=benjamin&email=benjamin%40gmail.com";
    let response = app.post_subscriptions(body.into()).await;
    // Assert
    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query_as!(SavedData, "SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");
    assert_eq!(saved.email, "benjamin@gmail.com");
    assert_eq!(saved.name, "benjamin");
    assert_eq!(saved.status, "confirmed");

    // BUG: using fetch can't close db.
    // https://github.com/launchbadge/sqlx/issues/1928
}

#[tokio::test]
async fn subscribe_returns_a_200_when_fields_are_present_but_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        ("name=&email=benjamin%40gmail.com", "empty name"),
        ("name=benjamin&email=", "empty email"),
        (
            "name=benjamin&email=definitely-not-an-email",
            "invalid email",
        ),
    ];

    for (body, description) in test_cases {
        // Act
        let response = app.post_subscriptions(body.into()).await;

        // Assert
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return a 200 OK when the payload was {}.",
            description
        );
    }
}
