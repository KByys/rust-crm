use axum::{http::HeaderMap, Json};
use mysql::{params, prelude::Queryable, PooledConn};
use serde_json::Value;

use crate::{
    bearer,
    database::{c_or_r, catch_some_mysql_error, get_conn, Database},
    debug_info,
    libs::time::{TimeFormat, TIME},
    pages::CUSTOM_FIELD_INFOS,
    parse_jwt_macro, Response, ResponseResult,
};

use super::{Customer, CUSTOMER_FIELDS};

pub async fn insert_customer(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    debug_info(format!("添加客户，操作者：{}， 数据: {:?}", id, value));
    let mut data: Customer = serde_json::from_value(value)?;
    data.fixed_infos.salesman = {
        let salesman: Option<String> = conn.query_first(format!(
            "SELECT id FROM user WHERE id = '{}'",
            data.fixed_infos.salesman
        ))?;
        salesman.unwrap_or(id.clone())
    };
    c_or_r(_insert, &mut conn, &data, false)?;

    Ok(Response::empty())
}

fn _insert(conn: &mut PooledConn, data: &Customer) -> Result<(), Response> {
    let fixed_infos = &data.fixed_infos;
    let create_time = TIME::now().unwrap().format(TimeFormat::YYYYMMDD_HHMMSS);
    conn.query_drop(format!(
        "INSERT INTO customer ({CUSTOMER_FIELDS}, visited_count, scope) VALUES (
        '{}', '{}', '{}', {}, {}, '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{create_time}', '{}', '{}', '{}', '{}', '{}', 0, 0)",
        fixed_infos.id,
        fixed_infos.name,
        fixed_infos.company,
        fixed_infos.is_share,
        fixed_infos.sex,
        fixed_infos.salesman,
        fixed_infos.chat,
        fixed_infos.next_visit_time,
        fixed_infos.need,
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
    )).map_err(|err|catch_some_mysql_error(Database::DUPLICATE_KEY_ERROR_CODE, "该客户已录入", err))?;
    // 添加下次预约
    if !fixed_infos.next_visit_time.is_empty() {
        conn.query_drop(format!(
            "INSERT INTO appointment (applicant, salesman, customer, appointment, status) 
            VALUES ('{}', '{}', '{}', '{}', {})",
            fixed_infos.salesman,
            fixed_infos.salesman,
            fixed_infos.id,
            fixed_infos.next_visit_time,
            0
        ))?;
    }
    let digest = md5::compute("12345678");
    //  注册客户账号
    conn.exec_drop(
        format!(
            "INSERT INTO customer_login (id, password) VALUES ('{}', :psw)",
            fixed_infos.id
        ),
        params! {
            "psw" => digest.0
        },
    )?;
    // 添加自定义字段的值
    for i in 0..3 {
        let table = CUSTOM_FIELD_INFOS[0][i];
        if data.custom_infos.get(i).is_empty() {
            continue;
        }
        conn.query_drop(format!(
            "INSERT INTO {table} (id, display, value) VALUES {}",
            data.custom_infos.generate_sql(i, &fixed_infos.id)
        ))?;
    }

    Ok(())
}
