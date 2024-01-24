use axum::{http::HeaderMap, routing::{get, post}, Json, Router};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use op::ternary;
use serde_json::{json, Value};

mod login;
mod logout;
mod register;
use crate::{
    bearer,
    database::get_conn,
    libs::{dser::*, time::TIME},
    parse_jwt_macro,
    perm::{action::OtherGroup, verify_permissions},
    Response, ResponseResult,
};
/// 员工数据
#[derive(Debug, serde::Serialize, FromRow, serde::Deserialize)]
#[mysql(table_name = "user")]
pub struct User {
    pub id: String,
    pub name: String,
    #[allow(unused)]
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub password: Vec<u8>,
    #[serde(default)]
    pub department: String,
    #[serde(deserialize_with = "deserialize_role")]
    #[serde(serialize_with = "serialize_role")]
    pub role: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    pub sex: i32,
}

pub fn account_router() -> Router {
    Router::new()
        .route("/user/login", post(login::user_login))
        .route("/root/register", post(register::register_root))
        .route("/user/data", post(query_user_data))
        .route("/customer/login", post(login::customer_login))
        .route("/user/register", post(register::register_user))
        .route("/user/set/psw", post(set_user_password))
        .route("/customer/set/psw", post(set_customer_password))
        .route("/role/infos", get(get_role))
}

async fn get_role() -> ResponseResult {
    let mut conn = get_conn()?;
    let roles = conn.query_map("SELECT name FROM roles WHERE id != 'root'", |s: String|s)?;
    Ok(Response::ok(json!(roles)))
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
pub fn get_user(id: &str, conn: &mut PooledConn) -> Result<User, Response> {
    let u: User = op::some!(conn.query_first(format!("SELECT * FROM user WHERE id = '{id}' LIMIT 1"))?; ret Err(Response::not_exist("用户不存在")));
    Ok(u)
}
async fn query_user_data(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    #[derive(serde::Deserialize)]
    struct Data {
        department: String,
        whole: bool,
    }
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: Data = serde_json::from_value(value)?;
    let u = get_user(&id, &mut conn)?;
    let perm = verify_permissions(&u.role, "other", OtherGroup::COMPANY_STAFF_DATA, None).await;
    let d = ternary!(data.department.is_empty() =>
        ternary!(perm => "?"; &u.department);
        ternary!(perm => &data.department; return Err(Response::permission_denied()))
    );

    if !data.whole {
        let count = conn.query_map(
            format!(
                "SELECT id FROM user WHERE department = '{}'",
                ternary!(d.eq("?") => &u.department; d)
            ),
            |s: String| s,
        )?;
        Ok(Response::ok(json!({"count": count.len()})))
    } else {
        let mut infos = Vec::new();
        if d.eq("?") {
            let departs = conn.query_map("SELECT value FROM department", |s: String| s)?;
            for d in departs {
                infos.push(query_user_data_by(&d, &id, &mut conn)?)
            }
        } else {
            infos.push(query_user_data_by(d, &id, &mut conn)?);
        }
        Ok(Response::ok(json!(infos)))
    }
}

fn query_user_data_by(d: &str, id: &str, conn: &mut PooledConn) -> mysql::Result<Value> {
    let data = conn.query_map(
        format!("SELECT * FROM user WHERE department = '{d}' AND id != '{id}' ORDER BY identity"),
        |user: User| user,
    )?;
    Ok(json!({
        "department": d,
        "data": data
    }))
}
