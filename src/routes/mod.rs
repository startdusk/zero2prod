mod health_check;
mod newsletters;
mod subscriptions;
mod subscriptions_confirm;

use actix_web::{
    http::{header, StatusCode},
    HttpResponse, ResponseError,
};
pub use health_check::*;
pub use newsletters::*;
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use reqwest::header::HeaderValue;
pub use subscriptions::*;
pub use subscriptions_confirm::*;

// A new error type, wrapping a sqlx::Error
#[derive(Debug)]
pub struct StoreTokenError(sqlx::Error);

impl std::fmt::Display for StoreTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::error::Error for StoreTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        // The compiler transparently casts `&sqlx::Error` into a `&dyn Error`
        Some(&self.0)
    }
}

impl ResponseError for StoreTokenError {}

#[derive(thiserror::Error)]
pub enum SubscriberError {
    #[error("{0}")]
    ValidationError(String),
    #[error("Failed to store the confirmation token for a new subscriber.")]
    StoreTokenError(#[from] StoreTokenError),
    #[error("Failed to send a confirmation email.")]
    SendEmailError(#[from] reqwest::Error),

    #[error("Failed to acquire a Postgres connection from the pool.")]
    PoolError(#[source] sqlx::Error),
    #[error("Failed to insert new subscriber in the database.")]
    InsertSubscriberError(#[source] sqlx::Error),
    #[error("Failed to commit SQL transaction to store a new subscriber.")]
    TransactionCommitError(#[source] sqlx::Error),
}

impl std::fmt::Debug for SubscriberError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for SubscriberError {
    fn status_code(&self) -> reqwest::StatusCode {
        match self {
            SubscriberError::ValidationError(_) => StatusCode::BAD_REQUEST,
            SubscriberError::PoolError(_)
            | SubscriberError::TransactionCommitError(_)
            | SubscriberError::InsertSubscriberError(_)
            | SubscriberError::StoreTokenError(_)
            | SubscriberError::SendEmailError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<String> for SubscriberError {
    fn from(e: String) -> Self {
        Self::ValidationError(e)
    }
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("Authentication failed.")]
    AuthError(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        match self {
            PublishError::AuthError(err) => {
                println!("{}", err);
                let mut resp = HttpResponse::new(StatusCode::UNAUTHORIZED);
                let header_value = HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();
                resp.headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);
                resp
            }
            PublishError::UnexpectedError(err) => {
                println!("{}", err);
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}

/// Generate a random 25-characters-long case-sensitive subscription token.
pub fn generate_subscription_token() -> String {
    let mut rng = thread_rng();
    std::iter::repeat_with(|| rng.sample(Alphanumeric))
        .map(char::from)
        .take(25)
        .collect()
}

fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{}\n", e)?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{}", cause)?;
        current = cause.source();
    }
    Ok(())
}
