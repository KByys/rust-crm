pub mod index;
mod appointment;
mod colleague;
use axum::Router;

use self::{appointment::appointment_router, colleague::colleague_router};


pub fn customer_router() -> Router {
    index::customer_router()
    .merge(colleague_router())
    .merge(appointment_router())
}