use crate::{
    bearer,
    database::get_db,
    libs::time::{TimeFormat, TIME},
    log,
    pages::account::get_user,
    parse_jwt_macro,
    perm::action::CustomerGroup,
    verify_perms, Response, ResponseResult, ID, SEA_MAX_DAY, SEA_MIN_DAY,
};
use axum::{
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use chrono::{prelude::TimeZone, Days};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde_json::{json, Value};

pub fn sea_router() -> Router {
    Router::new()
        .route("/sea/customer/infos", post(sea_infos))
        .route("/sea/push", post(push_customer_to_sea))
        .route("/sea/pop", post(pop_customer_from_sea))
        .route("/sea/set/permissions", post(set_sea_perm))
        .route("/sea/get/permissions", get(get_sea_perm))
}

async fn push_customer_to_sea(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let data: ID = serde_json::from_value(value)?;
    let sql_data: Option<(String, String)> = conn.exec_first(
        "select c.name, ex.salesman from customer c join extra_customer_data ex on ex.id=c.id where c.id  = ? limit 1", (&data.id, ),
    )?;

    let (customer_name, salesman) = match sql_data {
        Some(data) => data,
        _ => {
            log(format_args!(
                "{}({})释放客户{}到公海失败，没有该客户信息",
                user.id, user.name, data.id
            ));
            return Err(Response::not_exist("该客户不存在"));
        }
    };
    if salesman == user.id {
        let value = if data.public {
            if verify_perms!(
                &user.role,
                CustomerGroup::NAME,
                CustomerGroup::RELEASE_CUSTOMER
            ) {
                mysql::Value::NULL
            } else {
                log(format_args!(
                    "{}({})释放客户{}({})到公司公海失败，因为权限不足",
                    user.id, user.name, data.id, customer_name
                ));
                return Err(Response::permission_denied());
            }
        } else {
            mysql::Value::Bytes(user.department.clone().into_bytes())
        };
        let date = TIME::now()?;
        conn.exec_drop(
            "insert into customer_sea (id, put_in_date, sea) values (?, ?, ?)",
            (&data.id, date.format(TimeFormat::YYYYMMDD), value),
        )?;
        log(format_args!(
            "{}({})成功释放客户{}({})到{} 公海",
            user.id,
            user.name,
            data.id,
            customer_name,
            op::ternary!(data.public => "公司", &user.department)
        ));
        Ok(Response::empty())
    } else {

        log(format_args!(
            "{}({})释放客户{}({})到{} 公海失败，目前只能释放自己的客户",
            user.id,
            user.name,
            data.id,
            customer_name,
            op::ternary!(data.public => "公司", &user.department)
        ));
        Err(Response::permission_denied())
    }
}

async fn pop_customer_from_sea(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: ID = serde_json::from_value(value)?;
    let time = TIME::now()?;
    let local = chrono::Local.timestamp_nanos(time.naos() as i64);
    let min_deadline = unsafe {
        TIME::from(
            local
                .checked_sub_days(Days::new(SEA_MIN_DAY))
                .unwrap_or_default(),
        )
    };
    let key: Option<(String, Option<String>)> = conn.query_first(format!(
        "SELECT salesman, push_to_sea_date FROM customer 
        WHERE id = '{}' AND scope > 0",
        data.id
    ))?;

    if let Some((salesman, push_date)) = key {
        if salesman == id {
            if let Some(date) = push_date {
                if min_deadline.format(TimeFormat::YYYYMMDD_HHMM) > date {
                    return Err(Response::dissatisfy("未满足领取条件"));
                }
            }
        }
    } else {
        return Err(Response::not_exist("该客户不存在"));
    }
    conn.query_drop(format!(
        "UPDATE customer SET salesman = '{}', pop_from_sea_date = '{}', scope = 0 WHERE id = '{}'",
        id,
        time.format(TimeFormat::YYYYMMDD_HHMM),
        data.id,
    ))?;
    Ok(Response::empty())
}
#[allow(dead_code)]
#[derive(serde::Deserialize)]
struct Sea {
    sort: usize,
    scope: usize,
    department: String,
}
#[derive(serde::Serialize, FromRow)]
struct SeaInfo {
    name: String,
    company: String,
    ty: String,
    id: String,
    #[mysql(rename = "push_to_sea_date")]
    time: String,
}

async fn sea_infos(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let _id = parse_jwt_macro!(&bearer, &mut conn => true);
    let _data: Sea = serde_json::from_value(value)?;

    // let mut infos = match Identity::new(&id, &mut conn)? {
    //     Identity::Administrator(_, d) => query(&mut conn, &data, &d)?,
    //     Identity::Boss => query(&mut conn, &data, &data.department)?,
    //     Identity::Staff(_, d) => query(&mut conn, &data, &d)?,
    // };
    // use rust_pinyin::get_pinyin;
    // infos.sort_by(|v1, v2| match data.sort {
    //     0 => get_pinyin(&v1.name).cmp(&get_pinyin(&v2.name)),
    //     1 => get_pinyin(&v1.company).cmp(&get_pinyin(&v2.company)),
    //     2 => v1.time.cmp(&v2.time),
    //     _ => Ordering::Equal,
    // });
    // Ok(Response::ok(json!(infos)))
    todo!()
}

fn query(conn: &mut PooledConn, data: &Sea, d: &str) -> mysql::Result<Vec<SeaInfo>> {
    let infos = match data.scope {
        0 => {
            let mut infos = get_all(conn)?;
            let query = format!(
                "SELECT c.name, company, ty, c.id, push_to_sea_date FROM customer c 
                        JOIN user u ON u.id = c.salesman AND u.department = '{d}'
                            WHERE scope = 1"
            );
            infos.append(&mut conn.query_map(query, |s| s)?);
            infos
        }
        1 => get_all(conn)?,
        2 => {
            let query = format!(
                "SELECT c.name, company, ty, c.id, push_to_sea_date FROM customer c 
                        JOIN user u ON u.id = c.salesman AND u.department = '{d}'
                            WHERE scope = 1"
            );
            conn.query_map(query, |s| s)?
        }
        // 不存在
        _ => Vec::new(),
    };
    Ok(infos)
}

fn get_all(conn: &mut PooledConn) -> mysql::Result<Vec<SeaInfo>> {
    conn.query_map(
        "SELECT name, company, ty, id, push_to_sea_date FROM customer WHERE scope = 2",
        |s| s,
    )
}
#[derive(serde::Deserialize)]
struct SeaPerm {
    max_day: u64,
    min_day: u64,
}

async fn set_sea_perm(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    parse_jwt_macro!(&bearer, &mut conn => true);
    let data: SeaPerm = serde_json::from_value(value)?;
    unsafe {
        SEA_MIN_DAY = data.min_day;
        SEA_MAX_DAY = data.max_day;
        std::fs::write("data/sea", format!("{}-{}", SEA_MAX_DAY, SEA_MIN_DAY))?;
    }
    Ok(Response::empty())
}
async fn get_sea_perm() -> ResponseResult {
    unsafe {
        Ok(Response::ok(json!({
            "max_day": SEA_MAX_DAY,
            "min_day": SEA_MIN_DAY
        })))
    }
}
