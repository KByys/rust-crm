pub mod roles;
use std::collections::HashMap;

use crate::{bearer, database::get_conn, parse_jwt_macro, perm::roles::ROLE_TABLES, Response, ResponseResult};
use axum::{http::HeaderMap, routing::post, Router};
use mysql::{prelude::Queryable, PooledConn};
use serde_json::json;
use tokio::sync::Mutex;
pub type PermissionGroupMap = HashMap<String, HashMap<String, Vec<String>>>;
pub type PermissionMap = HashMap<String, Vec<String>>;
#[allow(elided_lifetimes_in_associated_constant)]
// #[forbid(unused)]
pub(crate) mod action;

// pub async fn get_role_id(name: &str) -> Option<String> {
//     let table = ROLE_TABLE.lock().await;
//     table.get(name).map(|s|s.to_string())
// }

// fn role_table() -> HashMap<String, String> {
//     let mut conn = get_conn().expect("初始化角色映射表时连接数据库失败");
//     conn.query_map("SELECT id, name FROM roles", |(id, name)| (id, name))
//         .expect("查询失败")
//         .into_iter()
//         .collect()
// }
// fn role_table_name() -> HashMap<String, String> {
//     let mut conn = get_conn().expect("初始化角色映射表时连接数据库失败");
//     conn.query_map("SELECT id, name FROM roles", |(id, name)| (name, id))
//         .expect("查询失败")
//         .into_iter()
//         .collect()
// }


// pub fn init_role_table() {

//     let mut conn = get_conn().expect("初始化角色映射表时连接数据库失败");
//     // let map = conn.query_map("SELECT id, name FROM roles", |(id, name)| (name, id))
//     //     .expect("查询失败");
// }



lazy_static::lazy_static! {
    // /// id -> name
    // pub static ref ROLE_TABLE: Mutex<HashMap<String, String>> = {
    //     Mutex::new(role_table())
    // };

    // /// name -> id
    // pub static ref ROLE_NAME_TABLE: Mutex<HashMap<String, String>> = {
    //     Mutex::new(role_table_name())
    // };
    pub static ref ROLES_GROUP_MAP: Mutex<HashMap<String, PermissionGroupMap>> = {
        let map = if let Ok(bytes) = std::fs::read("data/perm") {
            serde_json::from_slice(&bytes).expect("权限文件结构遭到破坏，请联系开发人员进行修复")
        } else {
            let mut map = HashMap::new();
            map.insert("salesman".to_owned(), role_salesman());
            map.insert("admin".to_owned(), unsafe { role_adm() });

            std::fs::write("data/perm", json!(map.clone()).to_string().as_bytes()).expect("写入权限文件失败");
            map
        };
        Mutex::new(map)
    };
}
pub async fn update_role_map(role: &str, perms: PermissionGroupMap) -> Result<(), Response> {
    use std::fs::write;
    let mut map = ROLES_GROUP_MAP.lock().await;
    if let Some(v) = map.get_mut(role) {
        *v = perms;
        write("data/perm", json!(map.clone()).to_string().as_bytes())?;
    }
    Ok(())
}
fn role_salesman() -> PermissionGroupMap {
    let mut map = HashMap::new();
    use action::*;
    map.insert("customer".to_string(), {
        CUSTOMER
            .iter()
            .filter_map(|f| {
                if matches!(
                    *f,
                    CustomerGroup::EXPORT_DATA
                        | CustomerGroup::RELEASE_CUSTOMER
                        | CustomerGroup::TRANSFER_CUSTOMER
                ) {
                    None
                } else {
                    Some((f.to_string(), vec![]))
                }
            })
            .collect()
    });
    map
}

unsafe fn role_adm() -> PermissionGroupMap {
    use action::*;
    let mut map = HashMap::new();
    map.insert(
        "customer".to_string(),
        CUSTOMER.iter().map(|x| (x.to_string(), vec![])).collect(),
    );
    map.insert("account".to_owned(), {
        let mut map = HashMap::new();
        map.insert(AccountGroup::CREATE.to_owned(), vec![ROLE_TABLES.get_name_uncheck("salesman")]);
        map.insert(AccountGroup::DELETE.to_owned(), vec![ROLE_TABLES.get_name_uncheck("salesman")]);
        map
    });
    map.insert("approval".to_owned(), {
        let mut map = HashMap::new();
        map.insert(ApprovalGroup::RECEIVE_APPROVAL.to_owned(), vec![]);
        map.insert(ApprovalGroup::QUERY_APPROVAL.to_owned(), vec![]);

        map
    });
    map.insert("other".into(), {
        let mut map = HashMap::new();
        map.insert(OtherGroup::QUERT_CHECK_IN.to_owned(), vec![]);

        map
    });
    map
}

pub fn perm_router() -> Router {
    Router::new().route("/get/perm", post(get_perm))
}
// id-name, mysql
// pub async fn check_permissions(role: &str, conn: &mut PooledConn) -> Result<(), Response> {
//     let mut role_maps = ROLES_GROUP_MAP.lock().await;
//     if role_maps.get(role).is_some() {
//         let path = op::some!(
//             conn.query_first::<String, String>(format!("SELECT perm FROM role WHERE role = '{role}'"))?; ret Err(Response::unknown_err("错误代码：perm_read: 10, 不应该发生")));
//         let data = tokio::fs::read(&path).await?;
//         let map = serde_json::from_slice(&data).map_err(|_| {
//             Response::internal_server_error("内部文件遭到损坏，请联系开发人员进行修复")
//         })?;
//         role_maps.insert(role.to_owned(), map);
//     }
//     Ok(())
// }

pub async fn verify_permissions(
    role: &str,
    perm: &str,
    action: &str,
    data: Option<&[&str]>,
) -> bool {
    if role.eq("root") {
        return true;
    }
    let role_perm_maps = ROLES_GROUP_MAP.lock().await;
    let role_perms = op::some!(role_perm_maps.get(role); ret false);

    op::some!(role_perms.get(perm); ret false)
        .get(action)
        .map_or(false, |v| {
            data.map_or(true, |d| d.iter().all(|k| v.contains(&k.to_string())))
        })
}




async fn get_perm(headers: HeaderMap) -> ResponseResult {
    let mut conn = get_conn()?;
    let bearer = bearer!(&headers);
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let role = get_role(&id, &mut conn)?;
    let perm_map = ROLES_GROUP_MAP.lock().await;
    if let Some(perms) = perm_map.get(&role) {
        Ok(Response::ok(json!(perms[&role])))
    } else {
        Ok(Response::ok(json!(PermissionGroupMap::new())))
    }
}
#[inline(always)]
pub fn get_role(id: &str, conn: &mut PooledConn) -> Result<String, Response> {
    let role = op::some!(conn.query_first(format!("SELECT role FROM user WHERE id = '{id}'"))?; ret Err(Response::not_exist("用户不存在")));
    Ok(role)
}

// async fn verify_perm(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
//     let bearer = bearer!(&headers);
//     let mut conn = get_conn()?;
//     let mut id = parse_jwt_macro!(&bearer, &mut conn => true);
//     let permissions: PermissionGroupMap = serde_json::from_value(value)?;
//     Ok(Response::ok(json!(true)))
// }
