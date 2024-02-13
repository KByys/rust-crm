use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde_json::{json, Value};

use crate::{
    bearer,
    database::get_conn,
    libs::{gen_id, time::TIME},
    parse_jwt_macro, Response, ResponseResult,
};

pub fn colleague_router() -> Router {
    Router::new()
    .route("/customer/colleague/data/:customer", get(query_colleagues))
    .route("/customer/colleague/insert/:customer", post(insert_colleague))
    .route("/customer/colleague/update", post(update_colleague))
    .route("/customer/colleague/delete/:id", delete(delete_colleague))
}

#[derive(serde::Deserialize, serde::Serialize, FromRow, Debug)]
struct Colleague {
    #[serde(default)]
    id: String,
    phone: String,
    name: String,
}
use super::index::check_user_customer;

async fn insert_colleague(
    headers: HeaderMap,
    Path(customer): Path<String>,
    Json(value): Json<Value>,
) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let mut params: Colleague = serde_json::from_value(value)?;
    check_user_customer(&id, &customer, &mut conn)?;
    let time = TIME::now()?;
    params.id = gen_id(&time, &params.name);
    conn.query_drop(format!(
        "INSERT INTO customer_colleague (id, customer, phone, name) VALUES (
        '{}', '{}', '{}', '{}')",
        params.id, customer, params.phone, params.name
    ))?;
    Ok(Response::empty())
}

async fn update_colleague(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: Colleague = serde_json::from_value(value)?;
    check(&id, &params.id, &mut conn)?;
    conn.query_drop(format!(
        "UPDATE customer_colleague SET phone = '{}', name = '{}' WHERE  id = '{}' LIMIT 1",
        params.phone, params.name, params.id,
    ))?;
    Ok(Response::empty())
}

fn check(id: &str, col: &str, conn: &mut PooledConn) -> Result<(), Response> {
    let query = format!(
        "SELECT 1 FROM user u
        JOIN extra_customer_data ex ON ex.salesman=u.id
         JOIN customer_colleague c ON c.customer = ex.id
         WHERE c.id='{col}' AND u.id = '{id}' LIMIT 1"
    );
    println!("{}", query);
    let flag: Option<String> = conn.query_first(query)?;
    if flag.is_some() {
        Ok(())
    } else {
        Err(Response::permission_denied())
    }
}

async fn delete_colleague(headers: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let user_id = parse_jwt_macro!(&bearer, &mut conn => true);
    check(&user_id, &id, &mut conn)?;
    conn.query_drop(format!(
        "DELETE FROM customer_colleague WHERE id = '{}' LIMIT 1",
        id,
    ))?;
    Ok(Response::empty())
}
async fn query_colleagues(Path(customer): Path<String>) -> ResponseResult {
    let mut conn = get_conn()?;
    let data: Vec<Colleague> = conn.query(format!(
        "SELECT id, name, phone FROM customer_colleague WHERE customer='{}'",
        customer
    ))?;
    Ok(Response::ok(json!(data)))
}
// #[derive(serde::Deserialize, serde::Serialize, FromRow, Debug)]
// struct ColleagueInfos {
//     customer_id: String,
//     #[serde(default)]
//     id: String,
//     #[serde(default)]
//     phone: String,
//     #[serde(default)]
//     name: String,
// }

// macro_rules! verify {
//     ($headers:expr, $value:expr) => {
//         {
//             let bearer = bearer!($headers);
//             let mut conn = get_conn()?;
//             let id = parse_jwt_macro!(&bearer, &mut conn => true);
//             let data: ColleagueInfos = serde_json::from_value($value)?;
//             let Some::<String>(salesman) = conn.query_first(format!("SELECT salesman FROM customer WHERE id = '{}'", data.customer_id))? else {
//                 return Err(Response::not_exist("该客户不存在"));
//             };
//             if salesman != id {
//                 return Err(Response::permission_denied());
//             }
//             (id, data, conn)
//         }
//     };
// }

// async fn add_colleague(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
//     let (_id, data, mut conn) = verify!(&headers, value);
//     if data.name.is_empty() || data.phone.is_empty() {
//         return Err(Response::invalid_value("phone或name不能为空字符串"));
//     }
//     let time = TIME::now()?;
//     let id = gen_id(&time, &data.name);
//     conn.query_drop(format!(
//         "INSERT INTO customer_colleague (customer_id, id, phone, name) VALUES ('{}', '{}', '{}', '{}')",
//         data.customer_id, id, data.phone, data.name
//     ))?;
//     Ok(Response::empty())
// }

// async fn update_colleague(
//     headers: HeaderMap,
//     Json(value): Json<serde_json::Value>,
// ) -> ResponseResult {
//     let (_id, data, mut conn) = verify!(&headers, value);
//     if data.name.is_empty() || data.phone.is_empty() {
//         return Err(Response::invalid_value("phone或name不能为空字符串"));
//     }
//     println!("{:?}", data);
//     let sta = format!(
//         "UPDATE customer_colleague SET phone = '{}', name = '{}' WHERE id = '{}' LIMIT 1",
//         data.phone, data.name, data.id
//     );
//     println!("{}", sta);
//     conn.query_drop(format!(
//         "UPDATE customer_colleague SET phone = '{}', name = '{}' WHERE id = '{}' LIMIT 1",
//         data.phone, data.name, data.id
//     ))?;
//     Ok(Response::empty())
// }

// async fn delete_colleague(
//     headers: HeaderMap,
//     Json(value): Json<serde_json::Value>,
// ) -> ResponseResult {
//     let (_id, data, mut conn) = verify!(&headers, value);
//     conn.query_drop(format!(
//         "DELETE FROM customer_colleague WHERE id = '{}' LIMIT 1",
//         data.id
//     ))?;
//     Ok(Response::empty())
// }

// async fn query_colleagues(Json(value): Json<serde_json::Value>) -> ResponseResult {
//     let Some(id) = crate::get_value(&value, "customer_id") else {
//         return Err(Response::invalid_format("缺失 customer_id"));
//     };
//     let mut conn = get_conn()?;
//     let data: Vec<ColleagueInfos> = conn.query_map(
//         format!(
//             "SELECT customer_id, id, phone, name FROM customer_colleague WHERE customer_id = '{}'",
//             id
//         ),
//         |c| c,
//     )?;
//     Ok(Response::ok(json!(data)))
// }
