use super::User;
use crate::libs::dser::*;
use crate::perm::roles::ROLE_TABLES;
use crate::perm::ROLES_GROUP_MAP;
use crate::{bearer, database::get_conn, debug_info, parse_jwt_macro, Response, ResponseResult};
use axum::{http::HeaderMap, Json};
use mysql::{params, prelude::Queryable, PooledConn};
use serde_json::{json, Value};

/// 12345678 的md5值
pub static DEFAULT_PASSWORD: [u8; 16] = [
    37, 213, 90, 210, 131, 170, 64, 10, 244, 100, 199, 109, 113, 60, 7, 173,
];

#[derive(serde::Deserialize)]
struct Root {
    id: String,
    password: String,
    name: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
}

pub async fn register_root(Json(value): Json<Value>) -> ResponseResult {
    let mut conn: PooledConn = get_conn()?;
    println!("{}", value);
    let root: Root = serde_json::from_value(value)?;
    let k: Option<String> = conn.query_first("SELECT id FROM user WHERE role = 'root'")?;
    if k.is_some() {
        return Err(Response::dissatisfy("只允许有一位最高权限者"));
    }
    conn.exec_drop(
        "INSERT INTO user (id, password, name,  sex, role, department) VALUES (
        :id, :password, :name, :sex, :role, :department
    )",
        params! {
            "id" => root.id,
            "password" => md5::compute(root.password.as_bytes()).0,
            "name" => root.name,
            "sex" => root.sex,
            "role" => "root",
            "department" => "总经办",
        },
    )?;
    Ok(Response::ok(json!({})))
}

// pub async fn root_register_all(Json(value): Json<Value>) -> ResponseResult {
//     let mut conn = get_conn()?;
//     let data: User = serde_json::from_value(value)?;
//     let digests = md5::compute("12345678");
//     println!("{:?}", data);
//     conn.exec_drop(
//         format!(
//             "INSERT INTO user (id, name, permissions, department, identity, sex, password)
//         VALUES ('{}', '{}', '{}', {}, '{}', '{}', :psw)",
//             data.id,
//             data.name,
//             data.permissions,
//             if data.identity > 0 {
//                 format!("'{}'", data.department)
//             } else {
//                 "'总经办'".to_string()
//             },
//             data.identity,
//             data.sex
//         ),
//         params! {"psw" => digests.0},
//     )?;
//     Ok(Response::empty())
// }

pub async fn register_user(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    println!("{:#?}", headers);
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    debug_info(format!("注册操作，操作者{}，数据:{:?}", id, value));
    let register_infos: User = serde_json::from_value(value)?;
    let is_exist: Option<String> = conn.query_first(format!(
        "SELECT id FROM user WHERE id = '{}'",
        register_infos.id
    ))?;
    if is_exist.is_some() {
        return Err(Response::already_exist(format!(
            "用户{} 已存在",
            register_infos.id
        )));
    }
    let (role, department) = op::some!(conn.query_first::<(String, String), String>(format!("SELECT role, department FROM user WHERE id = '{id}' LIMIT 1"))?; ret Err(Response::unknown_err("意外")));

    if role.eq("root") {
        if register_infos.department.eq("总经办") {
            return Err(Response::dissatisfy("总经办 这个部门不允许添加成员"));
        }
        insert_user(&register_infos, &mut conn)?;

        Ok(Response::ok(json!({})))
    } else if register_infos.department.eq(&department)
        && ver_user_cre_perm(&role, &register_infos).await
    {
        insert_user(&register_infos, &mut conn)?;
        Ok(Response::ok(json!({})))
    } else {
        Err(Response::permission_denied())
    }
}
/// 验证用户创建账号的权限
async fn ver_user_cre_perm(role: &str, infos: &User) -> bool {
    let perms = ROLES_GROUP_MAP.lock().await;
    perms
        .get(role)
        .and_then(|v| v.get("account").and_then(|f| f.get("create")))
        .map(|v| unsafe {
            ROLE_TABLES
                .get_name(&infos.role)
                .map_or(false, |role| v.contains(&role.to_owned()))
        })
        .unwrap_or(false)
}

// match Identity::new(&id, &mut conn)? {
//     Identity::Boss => match register_info.identity {
//         0 => register_root_user(&mut conn, &register_info),
//         1 | 2 => register_common_user(&mut conn, &register_info),
//         _ => Err(Response::invalid_value("identity的值错误")),
//     },
//     Identity::Administrator(_, depart) => match register_info.identity {
//         1 | 2 => register_common_user_with_depart(&mut conn, &register_info, &depart),
//         0 => Err(Response::permission_denied()),
//         _ => Err(Response::invalid_value("identity的值错误")),
//     },
//     _ => Err(Response::permission_denied()),
// }
#[inline]
fn insert_user(user: &User, conn: &mut PooledConn) -> mysql::Result<()> {
    conn.exec_drop(
        format!(
            "INSERT INTO user (id, name, password, department, role, sex) 
                VALUES('{}', '{}', :password, '{}', '{}', '{}')",
            user.id, user.name, user.department, user.role, user.sex
        ),
        params! {
            "password" => DEFAULT_PASSWORD,
        },
    )
}

// fn register_root_user(conn: &mut PooledConn, info: &User) -> ResponseResult {
//     let digest = md5::compute("12345678");
//     conn.exec_drop(
//         insert_statement(info, "总经办", 0),
//         params! {"password" => digest.0 },
//     )?;
//     Ok(Response::empty())
// }

// fn register_common_user(conn: &mut PooledConn, info: &User) -> ResponseResult {
//     let digest = md5::compute("12345678");
//     if info.department.as_str() == "总经办" {
//         return Err(Response::invalid_value(
//             "'总经办' 这个部门只允许最高权限者加入",
//         ));
//     }
//     // conn.exec_drop(
//     //     insert_statement(info, &info.department, info.permissions),
//     //     params! {"password" => digest.0 },
//     // )?;
//     Ok(Response::empty())
// }

// fn register_common_user_with_depart(
//     conn: &mut PooledConn,
//     info: &User,
//     department: &str,
// ) -> ResponseResult {
//     let digest = md5::compute("12345678");
//     if info.department.as_str() != department {
//         return Err(Response::permission_denied());
//     }

//     // conn.exec_drop(
//     //     insert_statement(info, department, info.permissions),
//     //     params! {"password" => digest.0 },
//     // )?;
//     Ok(Response::empty())
// }
