mod index;
#[allow(dead_code)]
mod reply;
use axum::Router;

pub fn report_router() -> Router {
    index::index_router()
    // .merge(reply::reply_router())
}
