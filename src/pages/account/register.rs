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
    println!("{:?}", data);
    conn.exec_drop(
        format!(
            "INSERT INTO user (id, name, permissions, department, identity, sex, password) 
        VALUES ('{}', '{}', '{}', {}, '{}', '{}', :psw)",
            data.id,
            data.name,
            data.permissions,
            if data.identity > 0 {
                format!("'{}'", data.department)
            } else {
                "'总经办'".to_string()
            },
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
fn insert_statement(user: &User, department: &str, perm: usize) -> String {
    format!(
        "INSERT INTO user (id, name, password, department, permissions, identity, sex) 
            VALUES('{}', '{}', :password, '{}', '{}', '{}', '{}')",
        user.id, user.name, department, perm, user.identity, user.sex
    )
}

fn register_root_user(conn: &mut PooledConn, info: &User) -> ResponseResult {
    let digest = md5::compute("12345678");
    conn.exec_drop(
        insert_statement(info, "总经办", 0),
        params! {"password" => digest.0 },
    )?;
    Ok(Response::empty())
}

fn register_common_user(conn: &mut PooledConn, info: &User) -> ResponseResult {
    let digest = md5::compute("12345678");
    if info.department.as_str() == "总经办" {
        return Err(Response::invalid_value(
            "'总经办' 这个部门只允许最高权限者加入",
        ));
    }
    conn.exec_drop(
        insert_statement(info, &info.department, info.permissions),
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
    if info.department.as_str() != department {
        return Err(Response::permission_denied());
    }

    conn.exec_drop(
        insert_statement(info, department, info.permissions),
        params! {"password" => digest.0 },
    )?;
    Ok(Response::empty())
}
