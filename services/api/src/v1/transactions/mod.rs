use axum::{routing::post, Router};

mod submit;

pub fn router() -> Router<crate::AppState> {
    Router::new().route("/submit", post(submit::handler))
}
