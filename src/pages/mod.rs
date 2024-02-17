mod user;
use axum::Router;

mod account;
mod form;
pub mod func;
mod message;
mod setting;
pub use setting::{
    option::{DropDownBox, DROP_DOWN_BOX, DROP_DOWN_BOX_ALL},
    CustomFields, Field, STATIC_CUSTOM_BOX_OPTIONS, STATIC_CUSTOM_FIELDS
};

pub fn pages_router() -> Router {
    account::account_router()
        .merge(setting::setting_router())
        .merge(message::message_router())
        .merge(func::func_router())
        .merge(user::user_router())
}
