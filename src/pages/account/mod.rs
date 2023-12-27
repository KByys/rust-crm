use axum::{http::HeaderMap, routing::post, Json, Router};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde_json::{Value, json};

mod login;
mod logout;
mod register;
use crate::{
    bearer,
    database::get_conn,
    libs::{dser::*, time::TIME, perm::Identity},
    parse_jwt_macro, Response, ResponseResult,
};
/// 员工数据
#[derive(Debug, serde::Serialize, FromRow, serde::Deserialize)]
#[mysql(table_name = "user")]
pub struct User {
    id: String,
    name: String,
    #[allow(unused)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    password: Vec<u8>,
    #[serde(default)]
    permissions: usize,
    #[serde(default)]
    department: String,
    identity: usize,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
}

pub fn account_router() -> Router {
    Router::new()
        .route("/user/login", post(login::user_login))
        .route("/root/register", post(register::root_register_all))
        .route("/user/data", post(query_user_data))
        .route("/customer/login", post(login::customer_login))
        .route("/user/register", post(register::register_user))
        .route("/user/set/psw", post(set_user_password))
        .route("/customer/set/psw", post(set_customer_password))
}
#[derive(serde::Deserialize)]
struct Password {
    password: String,
}
async fn set_customer_password(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => false);
    let password: Password = serde_json::from_value(value)?;
    let digest = md5::compute(password.password);
    let time = TIME::now()?;
    conn.exec_drop(
        "UPDATE customer_login SET password = :password WHERE id = :id",
        params! {
            "password" => digest.0,
            "id" => &id
        },
    )?;
    conn.query_drop(format!(
        "INSERT INTO token (ty, id, tbn) VALUES (1, '{}', {}) ON DUPLICATE KEY UPDATE tbn = {}",
        id,
        time.naos(),
        time.naos()
    ))?;
    Ok(Response::empty())
}
async fn set_user_password(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let password: Password = serde_json::from_value(value)?;
    let digest = md5::compute(password.password);
    let time = TIME::now()?;
    conn.exec_drop(
        "UPDATE user SET password = :password WHERE id = :id",
        params! {
            "password" => digest.0,
            "id" => &id
        },
    )?;
    conn.query_drop(format!(
        "INSERT INTO token (ty, id, tbn) VALUES (0, '{}', {}) ON DUPLICATE KEY UPDATE tbn = {}",
        id,
        time.naos(),
        time.naos()
    ))?;
    Ok(Response::empty())
}

async fn query_user_data(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    #[derive(serde::Deserialize)]
    struct Data {
        department: String,
        whole: bool 
    }
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: Data = serde_json::from_value(value)?;
    if !data.whole {
        let d = match Identity::new(&id, &mut conn)? {
            Identity::Boss if data.department.is_empty() => "总经办".to_owned(),
            Identity::Boss => data.department,
            Identity::Administrator(_, d) => d,
            Identity::Staff(_, d) => d
        };
        let count = conn.query_map(format!("SELECT id FROM user WHERE department = '{d}'"), |s: String|s)?;
        Ok(Response::ok(json!({"count": count.len()})))
    } else {
        
        let infos = match Identity::new(&id, &mut conn)? {
            Identity::Boss => {
                if data.department.is_empty() {
                    let mut infos = Vec::new();
                    let departs = conn.query_map("SELECT value FROM department", |s: String|s)?;
                    for d in departs {
                        infos.push(query_user_data_by(&d, &id, &mut conn)?)
                    }                
                    infos
                } else {
                    vec![query_user_data_by(&data.department, &id, &mut conn)?]
                }
            }
            Identity::Administrator(_, d) => vec![query_user_data_by(&d, &id, &mut conn)?],
            Identity::Staff(_, d) => vec![query_user_data_by(&d, &id, &mut conn)?]
        };

        Ok(Response::ok(json!(infos)))
    }
}

fn query_user_data_by(d: &str, id: &str, conn: &mut PooledConn) -> mysql::Result<Value> {
    let data = conn.query_map(
        format!("SELECT * FROM user WHERE department = '{d}' AND id != '{id}' ORDER BY identity"), |user: User|user)?;
    Ok(json!({
        "department": d,
        "data": data
    }))
}