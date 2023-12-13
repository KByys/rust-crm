use crate::{
    bearer, database::get_conn, debug_info, libs::perm::Identity, parse_jwt_macro, Response,
    ResponseResult,
};
use axum::{http::HeaderMap, Json};
use mysql::{params, prelude::Queryable, PooledConn};
use serde_json::Value;

use super::User;

pub async fn root_register_all(Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let data: User = serde_json::from_value(value)?;
    let digests = md5::compute("12345678");

    conn.exec_drop(
        format!(
            "INSERT INTO user (id, name, permissions, department, identity, sex, password) 
        VALUES ('{}', '{}', '{}', '{}', '{}', '{}', :psw)",
            data.id,
            data.name,
            data.permissions.unwrap(),
            data.department.unwrap(),
            data.identity,
            data.sex
        ),
        params! {"psw" => digests.0},
    )?;
    Ok(Response::empty())
}

pub async fn register_user(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    debug_info(format!("注册操作，操作者{}，数据:{:?}", id, value));
    let register_info: User = serde_json::from_value(value)?;
    let is_exist: Option<String> = conn.query_first(format!(
        "SELECT id FROM user WHERE id = '{}'",
        register_info.id
    ))?;
    if is_exist.is_some() {
        return Err(Response::already_exist(format!(
            "用户{} 已存在",
            register_info.id
        )));
    }
    match Identity::new(&id, &mut conn)? {
        Identity::Boss => match register_info.identity {
            0 => register_root_user(&mut conn, &register_info),
            1 | 2 => register_common_user(&mut conn, &register_info),
            _ => Err(Response::invalid_value("identity的值错误")),
        },
        Identity::Administrator(_, depart) => match register_info.identity {
            1 | 2 => register_common_user_with_depart(&mut conn, &register_info, &depart),
            0 => Err(Response::permission_denied()),
            _ => Err(Response::invalid_value("identity的值错误")),
        },
        _ => Err(Response::permission_denied()),
    }
}
#[inline]
fn insert_statement(user: &User, department: &str, perm: &str) -> String {
    format!(
        "INSERT INTO user (id, name, password, department, permissions, identity, sex) 
            VALUES('{}', '{}', :password, '{}', '{}', '{}', '{}')",
        user.id, user.name, department, perm, user.identity, user.sex
    )
}

fn register_root_user(conn: &mut PooledConn, info: &User) -> ResponseResult {
    let digest = md5::compute("12345678");
    conn.exec_drop(
        insert_statement(info, "NULL", "NULL"),
        params! {"password" => digest.0 },
    )?;
    Ok(Response::empty())
}

fn register_common_user(conn: &mut PooledConn, info: &User) -> ResponseResult {
    let digest = md5::compute("12345678");
    let depart = if let Some(depart) = &info.department {
        depart
    } else {
        return Err(Response::invalid_value("部门为空值"));
    };
    let perm = if let Some(perm) = info.permissions {
        perm.to_string()
    } else {
        return Err(Response::invalid_value("权限组为空值"));
    };

    conn.exec_drop(
        insert_statement(info, depart, &perm),
        params! {"password" => digest.0 },
    )?;
    Ok(Response::empty())
}

fn register_common_user_with_depart(
    conn: &mut PooledConn,
    info: &User,
    department: &str,
) -> ResponseResult {
    let digest = md5::compute("12345678");
    let depart = if let Some(depart) = &info.department {
        if depart != department {
            return Err(Response::permission_denied());
        }
        depart
    } else {
        return Err(Response::invalid_value("权限组为空值"));
    };
    let perm = if let Some(perm) = info.permissions {
        perm.to_string()
    } else {
        return Err(Response::invalid_value("权限组为空值"));
    };

    conn.exec_drop(
        insert_statement(info, depart, &perm),
        params! {"password" => digest.0 },
    )?;
    Ok(Response::empty())
}
