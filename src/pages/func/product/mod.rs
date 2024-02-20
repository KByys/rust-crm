mod index;
use axum::{extract::Multipart, http::HeaderMap, Router};

pub fn product_router() -> Router {
    Router::new()
}
