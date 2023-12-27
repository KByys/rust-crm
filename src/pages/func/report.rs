use std::cmp::Ordering;

use crate::{
    bearer,
    database::get_conn,
    do_if,
    libs::{
        gen_id,
        perm::Identity,
        time::{TimeFormat, TIME},
    },
    parse_jwt_macro, Response, ResponseResult,
};
use axum::{
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{
    params,
    prelude::{FromValue, Queryable},
    PooledConn,
};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Deserializer};
use serde_json::json;

pub fn report_router() -> Router {
    Router::new()
        .route("/report/add", post(add_report))
        .route("/report/send", post(send_report))
        .route("/report/read", post(process_report))
        .route("/report/update", post(update_report))
        .route("/report/delete", delete(delete_report))
        .route("/report/infos", post(query_reports))
        .route("/report/get/reply", get(get_report_replies))
}

#[derive(Debug, serde::Serialize, Default, Eq)]
struct User {
    name: String,
    phone: String,
}
impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}
impl PartialOrd for User {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.name.partial_cmp(&other.name)
    }
}
impl Ord for User {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}
impl User {
    fn name(&self) -> &str {
        &self.name
    }
}
impl<'de> serde::Deserialize<'de> for User {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name: String = serde::Deserialize::deserialize(deserializer)?;
        Ok(User::from(name))
    }
}
impl From<String> for User {
    fn from(phone: String) -> Self {
        User {
            phone,
            name: String::new(),
        }
    }
}
impl FromValue for User {
    type Intermediate = String;
}
#[derive(Debug, serde::Serialize, FromRow)]
struct ReportReply {
    id: String,
    contents: String,
    respondent: User,
    create_time: String,
    report_id: String,
}
fn deserialize_empty_to_none<'de, D>(de: D) -> Result<Option<User>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(de)?;
    Ok(value.and_then(|v| do_if!(v.is_empty() => None, Some(User::from(v)))))
}
#[derive(Debug, serde::Deserialize, serde::Serialize, FromRow)]
pub struct Report {
    #[serde(default)]
    id: String,
    #[serde(default)]
    applicant: User,
    reviewer: User,
    ty: usize,
    #[serde(default)]
    status: usize,
    #[serde(skip_deserializing)]
    create_time: String,
    #[serde(deserialize_with = "deserialize_empty_to_none")]
    cc: Option<User>,
    #[serde(deserialize_with = "deserialize_empty_to_none")]
    ac: Option<User>,
    contents: String,
    #[serde(skip_deserializing)]
    send_time: Option<String>,
    #[serde(skip_deserializing)]
    processing_time: Option<String>,
    #[serde(default)]
    opinion: Option<String>,
}
impl Report {
    fn ac(&self) -> Option<&User> {
        match &self.ac {
            Some(ac) => Some(ac),
            _ => None,
        }
    }
    fn cc(&self) -> Option<&User> {
        match &self.cc {
            Some(ac) => Some(ac),
            _ => None,
        }
    }
}

async fn add_report(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);

    let mut data: Report = serde_json::from_value(value)?;
    if !(0..3).contains(&data.ty) {
        return Err(Response::invalid_value("ty的值不对"));
    }
    data.applicant = User::from(id);
    let time = TIME::now()?;
    data.id = gen_id(
        &time,
        &format!("{}{}", rand::random::<char>(), rand::random::<char>()),
    );
    conn.exec_drop("INSERT INTO report (id, applicant, reviewer, ty, status, create_time, send_time, cc, ac, contents) 
        VALUES (:id, :applicant, :reviewer, :ty, :status, :create_time, :send_time, :cc, :ac, :contents)", params! {
            "id" => &data.id,
            "applicant" => data.applicant.name(),
            "reviewer" => data.reviewer.name(),
            "status" => do_if!(data.status == 1 => 1, 0),
            "ty" => data.ty,
            "create_time" => time.format(TimeFormat::YYYYMMDD_HHMMSS),
            "send_time" => time.format(TimeFormat::YYYYMMDD_HHMMSS),
            "cc" => data.cc().map_or(mysql::Value::NULL, |e|mysql::Value::Bytes(e.name.as_bytes().to_vec())),
            "ac" => data.ac().map_or(mysql::Value::NULL, |e|mysql::Value::Bytes(e.name.as_bytes().to_vec())),
            "contents" => &data.contents,
        })?;
    Ok(Response::empty())
}

async fn send_report(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let report_id: crate::ID = serde_json::from_value(value)?;
    let time = TIME::now()?;
    conn.query_drop(format!(
        "UPDATE report SET status = 1, send_time = '{}' WHERE id = '{}' AND applicant = '{}' LIMIT 1",
        time.format(crate::libs::time::TimeFormat::YYYYMMDD_HHMMSS), report_id.id, id
    ))?;
    Ok(Response::empty())
}

async fn process_report(
    headers: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    #[derive(serde::Deserialize)]
    struct Message {
        id: String,
        ok: bool,
        opinion: String,
    }
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: Message = serde_json::from_value(value)?;
    let status = do_if!(data.ok => 2, 3);
    let Some::<Report>(r) = conn.query_first(format!("SELECT * FROM report WHERE id = '{}'", data.id))? else {
        return Err(Response::not_exist("该报告不存在"));
    };
    if r.reviewer.phone != id {
        return Ok(Response::permission_denied());
    } else if r.status != 1 {
        return Err(Response::dissatisfy("该报告目前未发送或已被批阅"));
    }
    conn.query_drop(format!(
        "UPDATE report SET status = {status},  opinion = '{}' WHERE id = '{}' AND reviewer = '{}' LIMIT 1", data.opinion, data.id, id))?;
    Ok(Response::empty())
}
async fn update_report(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);

    let data: Report = serde_json::from_value(value)?;
    if !(0..3).contains(&data.ty) {
        return Err(Response::invalid_value("ty的值不对"));
    }
    let Some::<usize>(status) = conn.query_first(
            format!("SELECT status FROM report WHERE id = '{}' AND applicant = '{id}'", data.id))? else {
                return Err(Response::not_exist("找不到该报告"));
    };
    if status > 1 {
        return Err(Response::dissatisfy("只能修改未审批的报告"));
    }

    conn.exec_drop(format!("UPDATE report SET reviewer=:reviewer, ty=:ty, cc=:cc, ac=:ac, contents=:contents
        WHERE id = '{}' AND applicant = '{}' LIMIT 1 ", data.id, id), 
        params! {
            "reviewer" => data.reviewer.name(),
            "ty" => data.ty,
            "cc" => data.cc().map_or(mysql::Value::NULL, |e|mysql::Value::Bytes(e.name.as_bytes().to_vec())),
            "ac" => data.ac().map_or(mysql::Value::NULL, |e|mysql::Value::Bytes(e.name.as_bytes().to_vec())),
            "contents" => &data.contents 
        }
    )?;
    Ok(Response::empty())
}

#[derive(serde::Serialize)]
struct ResponseData {
    report: Report,
    replies: Vec<ReportReply>,
}
async fn query_reports(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    #[derive(serde::Deserialize)]
    struct Message {
        ty: usize,
        sort: usize,
        status: usize,
        applicant: String,
        reviewer: String,
        cc: String,
        ac: String,
    }
    let mut conn = get_conn()?;
    let bearer = bearer!(&headers);
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let perm = Identity::new(&id, &mut conn)?;
    let data: Message = serde_json::from_value(value)?;
    let mut filter = if data.ty <= 2 {
        format!("ty = {}", data.ty)
    } else {
        // 所有
        "ty >= 0".to_string()
    };
    match data.status {
        0 => filter.push_str(" AND status = 0"),
        1 => filter.push_str(" AND status = 1"),
        2 => filter.push_str(" AND status = 2"),
        3 => filter.push_str(" AND status = 3"),
        4 => (),
        _ => return Err(Response::ok(json!("status 非法"))),
    };
    match &perm {
        Identity::Boss => (),
        _ => {
            if data.applicant != id && data.reviewer != id && data.cc != id {
                return Err(Response::permission_denied());
            }
        }
    }
    if !data.applicant.is_empty() {
        filter.push_str(&format!("AND applicant = '{}'", data.applicant))
    }
    if !data.reviewer.is_empty() {
        filter.push_str(&format!("AND reviewer = '{}'", data.reviewer))
    }
    if !data.cc.is_empty() {
        filter.push_str(&format!("AND cc = '{}'", data.cc))
    }
    if !data.ac.is_empty() {
        filter.push_str(&format!("AND ac = '{}'", data.ac))
    }
    let reports: Vec<Report> =
        conn.query_map(format!("SELECT * FROM report WHERE {filter}"), |r| r)?;
    let mut res = Vec::new();
    for mut report in reports {
        let replies = get_replies(&report.id, &mut conn)?;
        report.applicant.name = conn
            .query_first(format!(
                "SELECT name FROM user WHERE id = '{}'",
                report.applicant.phone
            ))?
            .unwrap_or_default();
        report.reviewer.name = conn
            .query_first(format!(
                "SELECT name FROM user WHERE id = '{}'",
                report.reviewer.phone
            ))?
            .unwrap_or_default();
        report.ac = get_name(&mut conn, report.ac(), "customer");
        report.cc = get_name(&mut conn, report.cc(), "user");
        res.push(ResponseData { report, replies })
    }
    sort_reports(&mut res, data.sort);
    Ok(Response::ok(json!(res)))
}
fn get_name(conn: &mut PooledConn, u: Option<&User>, table: &str) -> Option<User> {
    let u = u?;
    conn.query_first(format!(
        "SELECT id, name FROM {table} WHERE id = '{}'",
        u.phone
    ))
    .ok()
    .and_then(|r| r.map(|(phone, name)| User { name, phone }))
}
fn sort_reports(data: &mut [ResponseData], sort: usize) {
    let sort = match sort {
        0 => |v1: &ResponseData, v2: &ResponseData| v1.report.send_time.cmp(&v2.report.send_time),
        1 => |v1: &ResponseData, v2: &ResponseData| {
            v1.report.processing_time.cmp(&v2.report.processing_time)
        },
        2 => {
            |v1: &ResponseData, v2: &ResponseData| v1.report.create_time.cmp(&v2.report.create_time)
        }
        3 => |v1: &ResponseData, v2: &ResponseData| {
            v1.report.applicant.name.cmp(&v2.report.applicant.name)
        },
        4 => |v1: &ResponseData, v2: &ResponseData| v1.report.reviewer.cmp(&v2.report.reviewer),
        5 => |v1: &ResponseData, v2: &ResponseData| v1.report.cc.cmp(&v2.report.cc),
        6 => |v1: &ResponseData, v2: &ResponseData| v1.report.ac.cmp(&v2.report.ac),
        7 => |v1: &ResponseData, v2: &ResponseData| v1.replies.len().cmp(&v2.replies.len()),
        8 => |v1: &ResponseData, v2: &ResponseData| {
            v1.replies
                .last()
                .map(|f| &f.create_time)
                .cmp(&v2.replies.last().map(|f| &f.create_time))
        },
        _ => |_v1: &ResponseData, _v2: &ResponseData| Ordering::Equal,
    };
    data.sort_by(sort);
}

async fn delete_report(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let report_id: crate::ID = serde_json::from_value(value)?;
    let mut conn = get_conn()?;
    let bearer = bearer!(&headers);
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    conn.query_drop(format!(
        "DELETE FROM report WHERE id = '{}' AND applicant = '{id}' AND status = 0 LIMIT 1",
        report_id.id
    ))?;
    Ok(Response::empty())
}
async fn get_report_replies(Json(value): Json<serde_json::Value>) -> ResponseResult {
    let id: crate::ID = serde_json::from_value(value)?;
    let mut conn = get_conn()?;
    let replies = get_replies(&id.id, &mut conn)?;
    Ok(Response::ok(json!(replies)))
}

fn get_replies(id: &str, conn: &mut PooledConn) -> mysql::Result<Vec<ReportReply>> {
    conn.query_map(
        format!("SELECT * FROM report_reply WHERE id = '{id}' ORDER BY create_time"),
        |r| r,
    )
}
