use axum::{routing::post, Router};

mod transactions;

pub fn router() -> Router<crate::AppState> {
    Router::new().nest("/transactions", transactions::router())
}
