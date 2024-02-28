use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    bearer,
    database::get_conn,
    libs::{gen_id, TimeFormat, TIME},
    pages::account::get_user,
    parse_jwt_macro, Response, ResponseResult,
};

pub fn reply_router() -> Router {
    Router::new()
        .route("/report/reply/add", post(add_reply))
        .route("/report/reply/update", post(update_reply))
        .route("/report/reply/delete/:id", delete(delete_reply))
        .route("/report/query/reply/:id", get(query_reply))
}
#[derive(Deserialize, Serialize, FromRow)]
pub struct Reply {
    #[serde(default)]
    id: String,
    #[serde(default)]
    create_time: String,
    #[serde(default)]
    applicant: String,
    #[serde(default)]
    applicant_name: String,
    contents: String,
    report: String,
}

async fn add_reply(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn)?;
    let params: Reply = serde_json::from_value(value)?;
    let time = TIME::now()?;
    let params = Reply {
        id: gen_id(&time, "reply"),
        create_time: time.format(TimeFormat::YYYYMMDD_HHMMSS),
        applicant: user.id,
        applicant_name: user.name,
        ..params
    };
    conn.query_drop(format!(
        "insert into report_reply (id, create_time, applicant, contents, report) values ('{}', '{}', '{}', '{}', '{}')",
        params.id, params.create_time, params.applicant, params.contents, params.report)
    )?;
    Ok(Response::ok(json!(params)))
}
#[derive(Deserialize)]
struct UpdateParams {
    id: String,
    contents: String,
}

async fn update_reply(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let param: UpdateParams = serde_json::from_value(value)?;
    conn.query_drop(format!(
        "update report_reply set contents = '{}' where id = '{}' and applicant='{uid}' limit 1",
        param.contents, param.id
    ))?;
    Ok(Response::empty())
}

async fn delete_reply(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    conn.query_drop(format!(
        "delete from report_reply where id = '{id}' and applicant = '{uid}' limit 1"
    ))?;
    Ok(Response::empty())
}

async fn query_reply(Path(id): Path<String>) -> ResponseResult {
    let mut conn = get_conn()?;
    let data = __query_reply(&mut conn, &id)?;
    Ok(Response::ok(json!(data)))
}

#[inline(always)]
pub fn __query_reply(conn: &mut PooledConn, id: &str) -> Result<Vec<Reply>, Response> {
    println!(
        "select r.*, a.name as applicant_name from report_reply r 
        join user a on a.id=r.applicant 
        where r.id='{id}' 
        order by r.create_time"
    );
    conn.query(format!(
        "select r.*, a.name as applicant_name from report_reply r 
        join user a on a.id=r.applicant 
        where r.report='{id}' 
        order by r.create_time"
    ))
    .map_err(Into::into)
}
