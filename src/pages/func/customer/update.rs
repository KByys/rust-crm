use axum::{http::HeaderMap, Json};
use mysql::{prelude::Queryable, PooledConn};
use serde_json::Value;

use crate::{
    bearer,
    database::{get_conn, Database},
    debug_info, parse_jwt_macro, Response, ResponseResult,
};
#[derive(serde::Deserialize)]
struct Info {
    id: String,
    data: FixedCustomerInfos,
}

use super::FixedCustomerInfos;

pub async fn update_customer_infos(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    debug_info(format!("更新客户信息，操作者：{}， 数据: {:?}", id, value));
    let info: Info = serde_json::from_value(value)?;
    let key: Option<String> = conn.query_first(format!(
        "SELECT id FROM customer WHERE id = '{}' AND salesman = '{}'",
        info.id, id
    ))?;
    if key.is_none() {
        return Err(Response::permission_denied());
    }
    conn.query_drop("BEGIN")?;
    conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    match _update(&mut conn, info) {
        Ok(_) => {
            conn.query_drop("COMMIT")?;
            conn.query_drop(Database::SET_FOREIGN_KEY_1)?;
            Ok(Response::empty())
        }
        Err(e) => {
            conn.query_drop("ROLLBACK")?;
            conn.query_drop(Database::SET_FOREIGN_KEY_1)?;
            Err(Response::internal_server_error(e))
        }
    }
}

fn _update(conn: &mut PooledConn, info: Info) -> mysql::Result<()> {
    let cus = &info.data;
    conn.query_drop(format!("UPDATE customer SET id = '{}', name = ' {}', company = '{}', is_share = {},
        sex = {}, chat = '{}', next_visit_time = '{}', need = '{}', fax == '{}', post = '{}', address = '{}',
        industry = '{}', birthday = '{}', remark = '{}', ty = '{}', tag = '{}', status = '{}', source = '{}', role = '{}'
        WHERE id = '{}'",
        cus.id, cus.name, cus.company, cus.is_share, cus.sex, cus.chat, cus.next_visit_time, cus.need,
        cus.fax, cus.post, cus.address, cus.industry, cus.birthday, cus.remark, cus.ty, cus.tag, cus.status,
        cus.source, cus.role, info.id
    ))?;
    if info.id != cus.id {
        conn.query_drop(format!(
            "UPDATE customer_login SET id = '{}' WHERE id = '{}'",
            cus.id, info.id
        ))?;
        conn.query_drop(format!(
            "UPDATE token SET id = '{}' WHERE id = '{}' AND ty = 1",
            cus.id, info.id
        ))?;
    }
    Ok(())
}
