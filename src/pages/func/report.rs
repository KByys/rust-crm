use std::cmp::Ordering;

use crate::{
    bearer,
    common::empty_deserialize_to_none,
    database::get_conn,
    libs::{
        gen_id,
        time::{TimeFormat, TIME},
    },
    parse_jwt_macro, Response, ResponseResult,
};
use axum::{
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
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

#[derive(Debug, serde::Serialize, mysql_common::prelude::FromRow)]
struct ReportReply {
    id: String,
    contents: String,
    respondent: String,
    respondent_name: String,
    create_time: String,
    report_id: String,
}
#[derive(Debug, serde::Deserialize, serde::Serialize, Default, mysql_common::prelude::FromRow)]
pub struct Report {
    #[serde(default)]
    id: String,

    #[serde(default)]
    applicant: String,
    #[serde(skip_deserializing)]
    applicant_name: String,

    reviewer: String,
    #[serde(skip_deserializing)]
    reviewer_name: String,

    ty: usize,
    #[serde(default)]
    status: usize,
    #[serde(skip_deserializing)]
    create_time: String,

    #[serde(deserialize_with = "empty_deserialize_to_none")]
    cc: Option<String>,
    #[serde(skip_deserializing)]
    cc_name: Option<String>,

    #[serde(deserialize_with = "empty_deserialize_to_none")]
    ac: Option<String>,
    #[serde(skip_deserializing)]
    ac_name: Option<String>,

    contents: String,
    #[serde(skip_deserializing)]
    send_time: Option<String>,
    #[serde(skip_deserializing)]
    processing_time: Option<String>,
    #[serde(default)]
    opinion: Option<String>,
}

async fn add_report(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);

    let mut data: Report = serde_json::from_value(value)?;
    if !(0..3).contains(&data.ty) {
        return Err(Response::invalid_value("ty的值不对"));
    }
    data.applicant = id;
    let time = TIME::now()?;
    data.id = gen_id(
        &time,
        &format!("{}{}", rand::random::<char>(), rand::random::<char>()),
    );
    conn.exec_drop("INSERT INTO report (id, applicant, reviewer, ty, status, create_time, send_time, cc, ac, contents) 
        VALUES (:id, :applicant, :reviewer, :ty, :status, :create_time, :send_time, :cc, :ac, :contents)", params! {
            "id" => &data.id,
            "applicant" => &data.applicant,
            "reviewer" => &data.reviewer,
            "status" => op::ternary!(data.status == 1 => 1; 0),
            "ty" => data.ty,
            "create_time" => time.format(TimeFormat::YYYYMMDD_HHMMSS),
            "send_time" => time.format(TimeFormat::YYYYMMDD_HHMMSS),
            "cc" => data.cc,
            "ac" => data.ac,
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
    let status = op::ternary!(data.ok => 2; 3);
    let Some::<Report>(r) =
        conn.query_first(format!("SELECT * FROM report WHERE id = '{}'", data.id))?
    else {
        return Err(Response::not_exist("该报告不存在"));
    };
    if r.reviewer != id {
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
    let Some::<usize>(status) = conn.query_first(format!(
        "SELECT status FROM report WHERE id = '{}' AND applicant = '{id}'",
        data.id
    ))?
    else {
        return Err(Response::not_exist("找不到该报告"));
    };
    if status > 1 {
        return Err(Response::dissatisfy("只能修改未审批的报告"));
    }

    conn.exec_drop(
        format!(
            "UPDATE report SET reviewer=:reviewer, ty=:ty, cc=:cc, ac=:ac, contents=:contents
        WHERE id = '{}' AND applicant = '{}' LIMIT 1 ",
            data.id, id
        ),
        params! {
            "reviewer" => &data.reviewer,
            "ty" => data.ty,
            "cc" => data.cc,
            "ac" => data.ac,
            "contents" => &data.contents
        },
    )?;
    Ok(Response::empty())
}

#[derive(serde::Serialize)]
struct ResponseData {
    report: Report,
    replies: Vec<ReportReply>,
}
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
macro_rules! change {
    (number $arg:expr, $slice:expr) => {
        op::ternary!($slice.contains(&$arg) => format!("={}", $arg); "IS NOT NULL".into())
    };
    (string $arg:expr, $null:expr, $name:expr) => {
        {
            if $arg.is_empty() {
                op::ternary!($null => format!("IS NOT NULL OR {} IS NULL", $name); "IS NOT NULL".into())
            } else {
                format!("= '{}'", $arg)
            }
        }
    };
}

fn _query_report(msg: &Message, conn: &mut PooledConn) -> mysql::Result<Vec<Report>> {
    conn.query_map(
        format!(
            "SELECT r.*, appr.name as applicant_name, rev.name as reviewer_name, 
        cc.name as cc_name, ac.name as ac_name 
        FROM report r
        LEFT JOIN user appr ON appr.id=r.applicant 
        LEFT JOIN user rev ON rev.id=r.reviewer
        LEFT JOIN customer ac ON ac.id=r.ac
        LEFT JOIN user cc ON cc.id=r.cc
        WHERE (
            (r.cc IS NULL AND r.ac IS NULL) OR
            (r.cc IS NOT NULL AND r.ac IS NULL) OR
            (r.cc IS NOT NULL AND r.ac IS NOT NULL)
        ) AND (r.ty {}) AND (r.status {}) AND (r.applicant {}) 
        AND (r.reviewer {}) AND (r.cc {}) AND (r.ac {})",
            change!(number msg.ty, 0..=2),
            change!(number msg.status, 0..=3),
            change!(string msg.applicant, false, ""),
            change!(string msg.reviewer, false, ""),
            change!(string msg.cc, true, "r.cc"),
            change!(string msg.ac, true, "r.ac"),
        ),
        |r| r,
    )
}

async fn query_reports(headers: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let bearer = bearer!(&headers);
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: Message = serde_json::from_value(value)?;
    if data.applicant != id && data.reviewer != id && data.cc != id {
        return Err(Response::permission_denied());
    }
    let reports = _query_report(&data, &mut conn)?;
    let mut res = Vec::new();
    for report in reports {
        let replies = get_replies(&report.id, &mut conn)?;
        res.push(ResponseData { report, replies })
    }
    sort_reports(&mut res, data.sort);
    Ok(Response::ok(json!(res)))
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
            v1.report.applicant_name.cmp(&v2.report.applicant_name)
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
        format!(
            "SELECT r.*, u.name as respondent_name FROM report_reply r 
            LEFT JOIN user u ON u.id=r.respondent WHERE r.id = '{id}' ORDER BY r.create_time"
        ),
        |r| r,
    )
}
