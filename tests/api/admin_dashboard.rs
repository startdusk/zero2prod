use crate::helpers::{spawn_app, assert_is_redirect_to};

#[tokio::test]
async fn you_must_be_logged_in_to_access_the_admin_dashboard() {
    // Arrange
    let app = spawn_app().await;

    // Act
    let resp = app.get_admin_dashborad().await;

    // Assert
    assert_is_redirect_to(&resp, "/login");
}