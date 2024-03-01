use crate::{
    bearer,
    database::{c_or_r, get_conn},
    libs::{dser::deser_empty_to_none, gen_id, TimeFormat, TIME},
    pages::account::get_user,
    parse_jwt_macro, Response, ResponseResult,
};
use axum::{
    extract::Path,
    http::HeaderMap,
    routing::{delete, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn index_router() -> Router {
    Router::new()
        .route("/report/add", post(add_report))
        .route("/report/send/:id", post(send_report))
        .route("/report/read", post(read_report))
        .route("/report/update", post(update_report))
        .route("/report/delete/:id", delete(delete_report))
        .route("/report/infos", post(query_report))
}

#[derive(Deserialize, Debug)]
struct InsertReportParams {
    ty: u8,
    reviewer: String,
    cc: Vec<String>,
    #[serde(deserialize_with = "deser_empty_to_none")]
    ac: Option<String>,
    contents: String,
    send: bool,
}

async fn add_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: InsertReportParams = serde_json::from_value(value)?;
    c_or_r(__insert_report, &mut conn, (&data, &uid), false)?;
    Ok(Response::empty())
}

fn __insert_report(
    conn: &mut PooledConn,
    (params, uid): (&InsertReportParams, &str),
) -> Result<(), Response> {
    let time = TIME::now()?;
    let id = gen_id(&time, "report");
    get_user(&params.reviewer, conn)?;
    println!("{:#?}", params);
    let send_time = op::ternary!(params.send =>
        mysql::Value::Bytes(time.format(TimeFormat::YYYYMMDD_HHMMSS).into_bytes()),
        mysql::Value::NULL
    );

    conn.exec_drop("INSERT INTO report (id, applicant, reviewer, ty, create_time, ac, contents,
    send_time, processing_time, opinion, status) VALUES (:id, :applicant, :reviewer, :ty, :create_time, :ac, :contents,
    :send_time, NULL, '', 2)", params! {
        "id" => &id,
        "applicant" => uid,
        "reviewer" => &params.reviewer,
        "ty" => params.ty,
        "create_time" => time.format(TimeFormat::YYYYMMDD_HHMMSS),
        "ac" => &params.ac,
        "contents" => &params.contents,
        "send_time" => send_time
    })?;
    for cc in &params.cc {
        conn.query_drop(format!(
            "INSERT IGNORE INTO report_cc (cc, report) VALUES ('{}', '{}')",
            cc, id
        ))?;
    }
    Ok(())
}

async fn send_report(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let send_time = TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS);
    conn.query_drop(format!("update report set send_time = '{send_time}' WHERE id = '{id}' AND applicant='{uid}' LIMIT 1"))?;
    Ok(Response::empty())
}
async fn delete_report(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let key: Option<String> = conn.query_first(format!(
        "select 1 from report where id = '{id}' and applicant='{uid}'"
    ))?;
    op::some!(key; ret Err(Response::permission_denied()));
    c_or_r(__delete_report, &mut conn, &id, false)?;
    Ok(Response::empty())
}
fn __delete_report(conn: &mut PooledConn, id: &str) -> Result<(), Response> {
    conn.query_drop(format!("delete from report where id = '{id}' LIMIT 1"))?;
    conn.query_drop(format!("delete from report_cc where report = '{id}'"))?;
    Ok(())
}

#[derive(Deserialize)]
struct ReadParams {
    id: String,
    ok: bool,
    opinion: String,
}

async fn read_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: ReadParams = serde_json::from_value(value)?;
    let status = op::ternary!(data.ok => 0, 1);
    let process_time = TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS);
    println!(
        "update report set status={status}, processing_time='{process_time}', opinion='{}' 
        WHERE id = '{}' AND reviewer='{uid}' AND send_time IS NOT NULL LIMIT 1",
        data.opinion, data.id
    );
    conn.query_drop(format!(
        "update report set status={status}, processing_time='{process_time}', opinion='{}' 
        WHERE id = '{}' AND reviewer='{uid}' AND send_time IS NOT NULL LIMIT 1",
        data.id, data.opinion
    ))?;

    Ok(Response::empty())
}
#[derive(Deserialize)]
struct UpdateParams {
    id: String,
    ty: i32,
    reviewer: String,
    cc: Vec<String>,
    ac: String,
    contents: String,
}
// #[derive(FromRow)]
// struct RowReport {
//     id: String,
//     reviewer: String,
//     ac: String,
//     processing_time: Option<String>
// }
async fn update_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: UpdateParams = serde_json::from_value(value)?;
    let key: Option<Option<String>> = conn.query_first(format!(
        "select processing_time from report where id = '{}' and applicant='{uid}'",
        data.id
    ))?;
    if let Some(r) = key {
        if r.is_none() {
            return Err(Response::dissatisfy("已批阅的报告无法修改"));
        }
    } else {
        return Err(Response::permission_denied());
    }
    __update_report(&mut conn, &data)?;
    Ok(Response::empty())
}

fn __update_report(conn: &mut PooledConn, param: &UpdateParams) -> Result<(), Response> {
    conn.query_drop(format!(
        "update report set ty={}, reviewer='{}', ac='{}', contents='{}'",
        param.ty, param.reviewer, param.ac, param.contents
    ))?;
    conn.query_drop(format!("delete from report_cc where report='{}'", param.id))?;
    for cc in &param.cc {
        conn.query_drop(format!(
            "insert ignore into report_cc (cc, report) values ('{}', '{}')",
            cc, param.id
        ))?;
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    ty: u8,
    status: u8,
    applicant: String,
    reviewer: String,
    cc: String,
    ac: String,
    limit: u32,
}

async fn query_report(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: QueryParams = serde_json::from_value(value)?;
    let reports = __query(&mut conn, &data, &uid)?;

    Ok(Response::ok(json!(reports)))
}
#[derive(mysql_common::prelude::FromRow, Serialize, Debug)]
struct Report {
    id: String,
    applicant: String,
    applicant_name: String,
    reviewer: String,
    reviewer_name: String,
    ac: Option<String>,
    ac_name: Option<String>,
    ty: i32,
    send_time: Option<String>,
    processing_time: Option<String>,
    opinion: String,
    contents: String,
    /// 0 审批通过，1 审批未通过，其他值表示未审批
    status: i32,
}

macro_rules! change {
    ($arg:expr, $name:expr) => {{
        if $arg.is_empty() {
            format!("is not null or {} is null", $name)
        } else {
            format!("='{}'", $arg)
        }
    }};
}

fn __query(conn: &mut PooledConn, params: &QueryParams, uid: &str) -> Result<Vec<Value>, Response> {
    println!("{:#?}", params);
    println!("uid - {}", uid);
    if !(params.cc.eq(uid) || params.applicant.eq(uid) || params.reviewer.eq(uid)) {
        return Err(Response::permission_denied());
    }
    let status = match params.status {
        0 => "r.send_time is not null and r.status=2",
        1 => "r.status=0",
        2 => "r.status=1",
        3 => "r.status is not null",
        _ => "r.send_time is null",
    };
    let ty = match params.ty {
        0..=2 => format!("= {}", params.ty),
        _ => "is not null".to_owned(),
    };
    let reviewer = change!(params.reviewer, "r.reviewer");
    let applicant = change!(params.applicant, "r.applicant");
    // let cc = change!(params.cc, "r.cc");
    let cc = if params.cc.is_empty() {
        String::new()
    } else {
        format!(
            "and exists(select 1 from report_cc where report=r.id and cc='{}')",
            params.cc
        )
    };
    let ac = change!(params.ac, "r.ac");
    let query = format!(
        "select r.*, a.name as applicant_name, rev.name as reviewer_name, c.name as ac_name from report r 
        join user a on r.applicant=a.id
        join user rev on rev.id = r.reviewer
        left join customer c on c.id = r.ac
        where (r.ty {ty}) and ({status}) and (r.ac {ac}) {cc} and (r.reviewer {reviewer}) and (r.applicant {applicant})
        limit {}
    ", params.limit);
    println!("{}", query);
    let mut reports = Vec::new();
    for row in conn.query::<Report, String>(query)? {
        println!("{:#?}", row);
        println!(
            "select rc.cc, u.name from report_cc rc 
                    join user u on u.id=rc.cc
                    where rc.report='{}'",
            row.id
        );
        let cc = conn.query_map(
            format!(
                "select rc.cc, u.name from report_cc rc 
                    join user u on u.id=rc.cc
                    where rc.report='{}'",
                row.id
            ),
            |(cc, name): (String, String)| {
                json!({
                    "name": name,
                    "id": cc
                })
            },
        )?;
        let replies = super::reply::__query_reply(conn, &row.id)?;
        reports.push(json!({
            "id": row.id,
            "applicant": row.applicant,
            "applicant_name": row.applicant_name,
            "reviewer": row.reviewer,
            "reviewer_name": row.reviewer_name,
            "ac": match row.ac {
                Some(ac) if !ac.is_empty() => Some(ac),
                _ => None
            },
            "ac_name": row.ac_name,
            "ty": row.ty,
            "send_time": row.send_time,
            "processing_time": row.processing_time,
            "opinion": row.opinion,
            "status": row.status,
            "cc": cc,
            "contents": row.contents,
            "replies": replies

        }));
    }
    Ok(reports)
}
