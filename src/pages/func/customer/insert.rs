use axum::{http::HeaderMap, Json};
use mysql::{params, prelude::Queryable, PooledConn};
use serde_json::Value;

use crate::{
    bearer,
    database::{get_conn, commit_or_rollback},
    debug_info,
    libs::time::{TimeFormat, TIME},
    parse_jwt_macro, Response, ResponseResult,
};

use super::{CustomInfos, FixedCustomerInfos, CUSTOMER_FIELDS};
#[derive(serde::Deserialize)]
struct ReceiveInfos {
    fixed_infos: FixedCustomerInfos,
    custom_infos: CustomInfos,
}

pub async fn insert_customer(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    debug_info(format!("添加客户，操作者：{}， 数据: {:?}", id, value));
    let mut data: ReceiveInfos = serde_json::from_value(value)?;
    data.fixed_infos.salesman = {
        let salesman: Option<String> = conn.query_first(format!(
            "SELECT id FROM user WHERE id = '{}'",
            data.fixed_infos.salesman
        ))?;
        if let Some(s) = salesman {
            s
        } else {
            id.clone()
        }
    };
    commit_or_rollback(_insert, &mut conn, &data, false)?;
    // let create_time = TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS);
    // conn.query_drop(format!(
    //     "INSERT INTO customer ({CUSTOMER_FIELDS}) VALUES (
    //     '{}', '{}', '{}', {}, {}, '{salesman}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{create_time}', '{}', '{}', '{}', '{}', '{}')",
    //     fixed_infos.id,
    //     fixed_infos.name,
    //     fixed_infos.company,
    //     fixed_infos.is_share,
    //     fixed_infos.sex,
    //     fixed_infos.chat,
    //     fixed_infos.next_visit_time,
    //     fixed_infos.fax,
    //     fixed_infos.post,
    //     fixed_infos.address,
    //     fixed_infos.industry,
    //     fixed_infos.birthday,
    //     fixed_infos.remark,
    //     fixed_infos.ty,
    //     fixed_infos.tag,
    //     fixed_infos.status,
    //     fixed_infos.source,
    //     fixed_infos.role
    // ))?;
    // let digest = md5::compute("12345678");
    // conn.exec_drop(
    //     format!(
    //         "INSERT INTO customer_login (id, password) VALUES ('{}', :psw)",
    //         fixed_infos.id
    //     ),
    //     params! {
    //         "psw" => digest.0
    //     },
    // )?;
    Ok(Response::empty())
}

fn _insert(conn: &mut PooledConn, data: &ReceiveInfos) -> mysql::Result<()> {
    
    let fixed_infos = &data.fixed_infos;
    // let salesman = {
    //     let salesman: Option<String> = conn.query_first(format!(
    //         "SELECT id FROM user WHERE id = '{}'",
    //         fixed_infos.salesman
    //     ))?;
    //     if let Some(s) = salesman {
    //         s
    //     } else {
    //         id.clone()
    //     }
    // };
    let create_time = TIME::now().unwrap().format(TimeFormat::YYYYMMDD_HHMMSS);
    conn.query_drop(format!(
        "INSERT INTO customer ({CUSTOMER_FIELDS}) VALUES (
        '{}', '{}', '{}', {}, {}, '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{create_time}', '{}', '{}', '{}', '{}', '{}')",
        fixed_infos.id,
        fixed_infos.name,
        fixed_infos.company,
        fixed_infos.is_share,
        fixed_infos.sex,
        fixed_infos.salesman,
        fixed_infos.chat,
        fixed_infos.next_visit_time,
        fixed_infos.fax,
        fixed_infos.post,
        fixed_infos.address,
        fixed_infos.industry,
        fixed_infos.birthday,
        fixed_infos.remark,
        fixed_infos.ty,
        fixed_infos.tag,
        fixed_infos.status,
        fixed_infos.source,
        fixed_infos.role
    ))?;
    let digest = md5::compute("12345678");
    conn.exec_drop(
        format!(
            "INSERT INTO customer_login (id, password) VALUES ('{}', :psw)",
            fixed_infos.id
        ),
        params! {
            "psw" => digest.0
        },
    )?;

    todo!();

    Ok(())
}