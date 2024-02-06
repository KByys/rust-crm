mod colleague;
mod delete;
mod insert;
mod query;
mod update;
use axum::{routing::post, Router};
use mysql_common::prelude::FromRow;

pub fn customer_router() -> Router {
    Router::new()
        .route("/customer/infos", post(query::qc_infos))
        .route("/customer/info/pages", post(query::qc_infos_with_pages))
        .route("/customer/add", post(insert::insert_customer))
        .route("/customer/update", post(update::update_customer_infos))
        .merge(colleague::colleague_router())
}

pub static CUSTOMER_FIELDS: &str =
    "id, name, company, is_share, sex, salesman, chat, next_visit_time, need, fax, post, address,
    industry, birthday, remark, create_time, ty, tag, status, source, role";

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Customer {
    fixed_infos: FCInfos,
    custom_infos: CCInfos,
}

use crate::{libs::dser::*, TextInfos};
/// 固定的客户信息
#[derive(Debug, serde::Deserialize, serde::Serialize, FromRow)]
pub struct FCInfos {
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
    /// 上次拜访时间
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub last_visited_time: Option<String>,
    /// 拜访次数
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub visited_count: i32,
    /// 上次成交时间
    #[serde(skip_deserializing)]
    #[serde(skip_serializing)]
    pub last_transaction_time: Option<String>,
}
/// 自定义的客户信息
#[derive(serde::Serialize, serde::Deserialize, Default, Debug)]
pub struct CCInfos {
    pub text_infos: Vec<TextInfos>,
    pub time_infos: Vec<TextInfos>,
    pub box_infos: Vec<TextInfos>,
}
impl CCInfos {
    pub fn get_mut(&mut self, i: usize) -> &mut Vec<TextInfos> {
        match i {
            0 => &mut self.text_infos,
            1 => &mut self.time_infos,
            2 => &mut self.box_infos,
            _ => unreachable!(),
        }
    }
    pub fn get(&self, i: usize) -> &[TextInfos] {
        match i {
            0 => &self.text_infos,
            1 => &self.time_infos,
            2 => &self.box_infos,
            _ => unreachable!(),
        }
    }
    pub fn generate_sql(&self, index: usize, id: &str) -> String {
        use std::fmt::Write as _;
        let mut sql: String = self
            .get(index)
            .iter()
            .fold(String::new(), |mut output, s| {
                let _ = write!(output, "('{}', '{}', '{}'),", id, s.display, s.value);
                output
            });
        sql.pop();
        sql
    }
}
