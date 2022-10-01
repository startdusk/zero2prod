use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn an_error_flash_message_is_set_on_failure() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let login_body = serde_json::json!({
        "username": "random-username",
        "password": "random-password",
    });
    let resp = app.post_login(&login_body).await;

    // Assert
    assert_eq!(resp.status().as_u16(), 303);

    assert_is_redirect_to(&resp, "/login");
    let flash_cookie = &resp.cookies().find(|c| c.name() == "_flash").unwrap();
    assert_eq!(flash_cookie.value(), "Authentication failed");
}
