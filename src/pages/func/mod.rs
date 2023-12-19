mod appointment;
mod sea;
use axum::Router;

mod customer;

pub fn func_router() -> Router {
    customer::customer_router().merge(sea::sea_router())
}
