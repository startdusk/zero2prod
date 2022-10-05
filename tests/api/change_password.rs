use crate::helpers::{assert_is_redirect_to, spawn_app};
use uuid::Uuid;

#[tokio::test]
async fn you_must_be_logged_in_to_see_the_change_password_form() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let resp = app.get_change_password().await;

    // Assert
    assert_is_redirect_to(&resp, "/login")
}

#[tokio::test]
async fn you_must_be_logged_in_to_change_your_passwword() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    // Assert
    assert_is_redirect_to(&resp, "/login")
}

#[tokio::test]
async fn new_password_fields_must_match() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let another_new_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Act - Part 2 - Try to change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
        "new_password": &new_password,
        "new_password_check": &another_new_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    // Act - Part 3 - Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains(
        "<p><i>You entered two different new passwords - \
        the field values must match.</i></p>"
    ))
}

#[tokio::test]
async fn current_password_must_be_valid() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Act - Part 2 - Try to change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": &new_password,
            "new_password_check": &new_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    // Act - Part 3 - Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>The current password is incorrect.</i></p>"))
}

#[tokio::test]
async fn new_password_must_be_valid() {
    // Arrange
    let app = spawn_app().await;
    //=====================================================================================
    // Too short(11 characters)
    let invalid_password = "12345678901".to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Act - Part 2 - Try to change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &invalid_password,
            "new_password_check": &invalid_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    // Act - Part 3 - Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>new passwords should be longer than 12 characters but shorter than 128 characters.</i></p>"));

    //=====================================================================================
    // Too long(129 characters)
    let invalid_password = "1234567890-1234567890-1234567890-1234567890-1234567890-1234567890-1234567890-1234567890-1234567890-1234567890-1234567890-1234567890".to_string();

    // Act - Part 1 - Login
    app.post_login(&serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // Act - Part 2 - Try to change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &invalid_password,
            "new_password_check": &invalid_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    // Act - Part 3 - Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>new passwords should be longer than 12 characters but shorter than 128 characters.</i></p>"))
}

#[tokio::test]
async fn changing_password_works() {
    // Arrange
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");

    // Act - Part 2 - Change password
    let resp = app
        .post_change_password(&serde_json::json!(
            {
                "current_password": app.test_user.password,
                "new_password": &new_password,
                "new_password_check": &new_password,
            }
        ))
        .await;
    assert_is_redirect_to(&resp, "/admin/password");

    // Act - Part 3 - Follow the redirect
    let html_page = app.get_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    // Act - Part 4 - Logout
    let resp = app.post_logout().await;
    assert_is_redirect_to(&resp, "/login");

    // Act - Part 5 - Follow the redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains("<p><i>You have successfully logged out.</i></p>"));

    // Act - Part 6 - Login using the new password
    let login_body = serde_json::json!({
        "username": app.test_user.username,
        "password": &new_password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");
}
