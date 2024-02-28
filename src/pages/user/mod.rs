use std::collections::HashMap;

use axum::{extract::Path, http::HeaderMap, routing::post, Json, Router};
use mysql::prelude::Queryable;
use serde_json::json;

use crate::{bearer, database::get_conn, parse_jwt_macro, Response, ResponseResult};

use super::account::User;

pub fn user_router() -> Router {
    Router::new().route("/user/name/:id", post(get_user_name))
    .route("/user/list/limit", post(query_limit_user))
}

async fn get_user_name(Path(id): Path<String>) -> ResponseResult {
    let mut conn = get_conn()?;
    let name: Option<String> =
        conn.query_first(format!("SELECT name FROM user WHERE id = '{id}' LIMIT 1"))?;
    Ok(Response::ok(json!(name)))
}

#[derive(serde::Deserialize)]
struct LimitParams {
    customer: String,
}
async fn query_limit_user(header: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let _uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: LimitParams = serde_json::from_value(value)?;
    // TODO 后面需要考虑共享情况
    let users: Vec<User> = conn.query(format!(
        "select u.* from user u 
        join extra_customer_data ex on ex.id='{}' and ex.salesman=u.id", data.customer))?;
    let mut map: HashMap<String, Vec<User>> = HashMap::new();
    for u in users {
        map.entry(u.department.clone()).or_default().push(u);
    }

    let values: Vec<serde_json::Value> = map.into_iter().map(|(k, v)| {
        serde_json::json!({
            "department": k,
            "data": v
        })
    }).collect();

    Ok(Response::ok(serde_json::json!(values)))

}