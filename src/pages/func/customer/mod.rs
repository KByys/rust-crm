mod delete;
mod insert;
mod query;
mod update;
use axum::{routing::post, Router};
use mysql_common::prelude::FromRow;

pub fn customer_router() -> Router {
    Router::new()
        .route("/customer/infos", post(query::query_all_customer_infos))
        .route("/customer/add", post(insert::insert_customer))
        .route("/customer/update", post(update::update_customer_infos))
}

pub static CUSTOMER_FIELDS: &str =
    "id, name, company, is_share, sex, salesman, chat, next_visit_time, need, fax, post, address,
    industry, birthday, remark, create_time, ty, tag, status, source, role";

use crate::{libs::dser::*, TextInfos};
#[derive(Debug, serde::Deserialize, serde::Serialize, FromRow)]
pub struct FixedCustomerInfos {
    pub id: String,
    pub name: String,
    pub company: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    pub is_share: i32,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    pub sex: i32,
    pub salesman: String,
    pub chat: String,
    pub next_visit_time: String,
    pub need: String,
    pub fax: String,
    /// 邮编
    pub post: String,
    pub address: String,
    pub industry: String,
    pub birthday: String,
    pub remark: String,
    #[serde(default)]
    pub create_time: String,
    pub ty: String,
    pub tag: String,
    pub status: String,
    pub source: String,
    pub role: String,
}
#[derive(serde::Serialize, serde::Deserialize)]
pub struct CustomInfos {
    text_infos: Vec<TextInfos>,
    time_infos: Vec<TextInfos>,
    box_infos: Vec<TextInfos>,
}
