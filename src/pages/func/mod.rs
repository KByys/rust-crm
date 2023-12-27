mod appointment;
mod product;
mod report;
mod sea;
use axum::Router;

mod customer;

pub fn func_router() -> Router {
    customer::customer_router()
        .merge(sea::sea_router())
        .merge(product::product_router())
        .merge(report::report_router())
        .merge(report::report_router())
}
