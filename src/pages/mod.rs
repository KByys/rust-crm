use axum::Router;

mod account;
mod form;
mod func;
mod message;
mod setting;
pub use setting::{DataOptions, CUSTOM_BOX_FIELDS, CUSTOM_FIELDS, CUSTOM_FIELD_INFOS};

pub fn pages_router() -> Router {
    account::account_router()
        .merge(setting::setting_router())
        .merge(message::message_router())
        .merge(func::func_router())
}
