mod appointment;
use axum::Router;

mod customer;

pub fn func_router() -> Router {
    customer::customer_router()
}
