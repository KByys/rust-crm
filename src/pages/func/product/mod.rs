mod index;
use axum::Router;

pub fn product_router() -> Router {
    Router::new().merge(index::product_router())
}
