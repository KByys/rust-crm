use axum::{
    http::HeaderMap,
    routing::{delete, post},
    Json, Router,
};
use mysql::prelude::Queryable;
use mysql_common::prelude::FromRow;
use serde_json::json;

use crate::{
    bearer,
    database::get_conn,
    libs::{gen_id, time::TIME},
    parse_jwt_macro, Response, ResponseResult,
};

pub fn colleague_router() -> Router {
    Router::new()
        .route("/customer/colleague/info", post(query_colleagues))
        .route("/customer/colleague/insert", post(add_colleague))
        .route("/customer/colleague/update", post(update_colleague))
        .route("/customer/colleague/delete", delete(delete_colleague))
}
#[derive(serde::Deserialize, serde::Serialize, FromRow, Debug)]
struct ColleagueInfos {
    customer_id: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    phone: String,
    #[serde(default)]
    name: String,
}

macro_rules! verify {
    ($headers:expr, $value:expr) => {
        {
            let bearer = bearer!($headers);
            let mut conn = get_conn()?;
            let id = parse_jwt_macro!(&bearer, &mut conn => true);
            let data: ColleagueInfos = serde_json::from_value($value)?;
            let Some::<String>(salesman) = conn.query_first(format!("SELECT salesman FROM customer WHERE id = '{}'", data.customer_id))? else {
                return Err(Response::not_exist("该客户不存在"));
            };
            if salesman != id {
                return Err(Response::permission_denied());
            }
            (id, data, conn)
        }
    };
}

async fn add_colleague(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let (_id, data, mut conn) = verify!(&headers, value);
    if data.name.is_empty() || data.phone.is_empty() {
        return Err(Response::invalid_value("phone或name不能为空字符串"));
    }
    let time = TIME::now()?;
    let id = gen_id(&time, &data.name);
    conn.query_drop(format!(
        "INSERT INTO customer_colleague (customer_id, id, phone, name) VALUES ('{}', '{}', '{}', '{}')",
        data.customer_id, id, data.phone, data.name
    ))?;
    Ok(Response::empty())
}

async fn update_colleague(
    headers: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let (_id, data, mut conn) = verify!(&headers, value);
    if data.name.is_empty() || data.phone.is_empty() {
        return Err(Response::invalid_value("phone或name不能为空字符串"));
    }
    println!("{:?}", data);
    let sta = format!(
        "UPDATE customer_colleague SET phone = '{}', name = '{}' WHERE id = '{}' LIMIT 1",
        data.phone, data.name, data.id
    );
    println!("{}", sta);
    conn.query_drop(format!(
        "UPDATE customer_colleague SET phone = '{}', name = '{}' WHERE id = '{}' LIMIT 1",
        data.phone, data.name, data.id
    ))?;
    Ok(Response::empty())
}

async fn delete_colleague(
    headers: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let (_id, data, mut conn) = verify!(&headers, value);
    conn.query_drop(format!(
        "DELETE FROM customer_colleague WHERE id = '{}' LIMIT 1",
        data.id
    ))?;
    Ok(Response::empty())
}

async fn query_colleagues(Json(value): Json<serde_json::Value>) -> ResponseResult {
    let Some(id) = crate::get_value(&value, "customer_id") else {
        return Err(Response::invalid_format("缺失 customer_id"));
    };
    let mut conn = get_conn()?;
    let data: Vec<ColleagueInfos> = conn.query_map(
        format!(
            "SELECT customer_id, id, phone, name FROM customer_colleague WHERE customer_id = '{}'",
            id
        ),
        |c| c,
    )?;
    Ok(Response::ok(json!(data)))
}
