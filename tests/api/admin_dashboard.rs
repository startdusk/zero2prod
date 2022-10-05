use crate::helpers::{assert_is_redirect_to, spawn_app};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let resp = app.get_admin_dashborad().await;

    // Assert
    assert_is_redirect_to(&resp, "/login");
}

#[tokio::test]
async fn logout_clears_session_state() {
    // Arrange
    let app = spawn_app().await;

    // Act - Part 1 - Login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");

    // Act - Part 2 - Follw the redirect
    let html_page = app.get_admin_dashborad_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // Act - Part 3 - Logout
    let resp = app.post_logout().await;
    assert_is_redirect_to(&resp, "/login");

    // Act - Part 4 - Follow the redirect
    let html_page = app.get_login_html().await;
    assert!(html_page.contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // Act - Part 5 - Attempt to load admin admin pannel
    let resp = app.get_admin_dashborad().await;
    assert_is_redirect_to(&resp, "/login")
}
