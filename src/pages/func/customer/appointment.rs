use axum::extract::Path;
use axum::routing::{delete, post};
use axum::{http::HeaderMap, Json, Router};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::libs::dser::deser_yyyy_mm_dd_hh_mm_ss;
use crate::libs::TimeFormat;
use crate::{
    bearer,
    database::get_conn,
    libs::{gen_id, TIME},
    parse_jwt_macro, Response, ResponseResult,
};

use super::index::check_user_customer;

pub fn appointment_router() -> Router {
    Router::new()
        .route("/customer/appointment/add", post(add_appointments))
        .route(
            "/customer/appointment/delete/:id",
            delete(delete_appointment),
        )
        .route("/customer/appointment/finish/:id", post(finish_appointment))
        .route(
            "/customer/appointment/data/:id/:limit",
            post(query_appointment),
        )
        .route("/customer/appoint/comment/add", post(insert_comment))
        .route("/customer/appoint/comment/update", post(update_comment))
        .route(
            "/customer/appoint/comment/delete/:id",
            delete(delete_comment),
        )
}
#[derive(Debug, Deserialize)]
struct InsertParams {

    #[serde(rename = "visitor")]
    salesman: String,
    customer: String,
    #[serde(deserialize_with = "deser_yyyy_mm_dd_hh_mm_ss")]
    appointment: String,
    theme: String,
    content: String,
    #[allow(dead_code)]
    #[serde(default)]
    notify: bool,
}

async fn add_appointments(
    header: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let user_id = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: Vec<InsertParams> = serde_json::from_value(value)?;
    for param in params {
        let time = TIME::now()?;
        check_user_customer(&user_id, &param.customer, &mut conn)?;
        let id = gen_id(&time, &rand::random::<i32>().to_string());
        conn.query_drop(format!(
            "INSERT INTO appointment 
            (id, customer, applicant, salesman, appointment, finish_time, theme, content) VALUES (
                '{}', '{}', '{}', '{}', '{}', NULL, '{}', '{}'
            )",
            id,
            param.customer,
            user_id,
            param.salesman,
            param.appointment,
            param.theme,
            param.content
        ))?;
    }
    Ok(Response::empty())
}
fn check(id: &str, app: &str, conn: &mut PooledConn) -> Result<(), Response> {
    let query = format!(
        "SELECT 1 FROM user u
        JOIN extra_customer_data ex ON ex.salesman=u.id
         JOIN appointment ap ON ap.customer = ex.id
         WHERE ap.id='{app}' AND u.id = '{id}' LIMIT 1"
    );
    println!("{}", query);
    let flag: Option<String> = conn.query_first(query)?;
    if flag.is_some() {
        Ok(())
    } else {
        Err(Response::permission_denied())
    }
}
async fn delete_appointment(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let user_id = parse_jwt_macro!(&bearer, &mut conn => true);
    check(&user_id, &id, &mut conn)?;
    conn.query_drop(format!(
        "DELETE FROM appointment WHERE id = '{}' LIMIT 1",
        id,
    ))?;
    Ok(Response::empty())
}

async fn finish_appointment(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let user_id = parse_jwt_macro!(&bearer, &mut conn => true);
    check(&user_id, &id, &mut conn)?;
    let time = TIME::now()?;
    let finish_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    conn.query_drop(format!(
        "UPDATE appointment SET finish_time = '{}' WHERE id = '{}' LIMIT 1",
        finish_time,
        id
    ))?;
    Ok(Response::ok(json!(finish_time)))
}
#[derive(Debug, Serialize, FromRow)]
struct AppointmentResponse {
    id: String,
    salesman: String,
    salesman_name: String,
    applicant: String,
    applicant_name: String,
    appointment: String,
    finish_time: Option<String>,
    theme: String,
    content: String,
}

fn join_to_json(appoint: &AppointmentResponse, comments: &[Comment]) -> Value {
    json!({
        "id": appoint.id,
        "visitor": appoint.salesman,
        "visitor_name": appoint.salesman_name,
        "applicant": appoint.applicant,
        "applicant_name": appoint.applicant_name,
        "appointment": appoint.appointment,
        "finish_time": appoint.finish_time,
        "theme": appoint.theme,
        "content": appoint.content,
        "comments": comments
    })
}

#[derive(Serialize, FromRow)]
struct Comment {
    applicant: String,
    applicant_name: String,
    id: String,
    appoint: String,
    create_time: String,
    comment: String,
}

async fn query_appointment(Path((id, limit)): Path<(String, usize)>) -> ResponseResult {
    let mut conn = get_conn()?;
    let res: Vec<AppointmentResponse> = conn.query(format!(
        "SELECT app.*, a.name as applicant_name, s.name as salesman_name FROM appointment app
        JOIN user a ON a.id = app.applicant
        JOIN user s ON s.id = app.salesman
        WHERE app.customer = '{}' ORDER BY appointment DESC LIMIT {limit}",
        id
    ))?;
    let mut data = Vec::new();
    for a in res {
        let comments = conn.query(format!(
            "SELECT com.*, a.name as applicant_name FROM appoint_comment com 
            JOIN user a ON a.id = com.applicant
            WHERE com.appoint = '{}'",
            a.id
        ))?;
        data.push(join_to_json(&a, &comments));
    }

    Ok(Response::ok(json!(data)))
}
#[derive(Debug, Deserialize)]
struct InsertCommentParams {
    comment: String,
    appoint: String,
}

async fn insert_comment(header: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: InsertCommentParams = serde_json::from_value(value)?;
    let time = TIME::now()?;
    let id = gen_id(&time, "comment");
    let create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    conn.query_drop(format!(
        "INSERT INTO appoint_comment (id, applicant, appoint, create_time, comment) VALUES (
        '{id}', '{uid}', '{}', '{}', '{}'
    )",
        data.appoint, create_time, data.comment
    ))?;
    let name: Option<String> =
        conn.query_first(format!("select name from user where id = '{uid}' limit 1"))?;
    Ok(Response::ok(json!({
        "applicant": uid,
        "applicant_name": name,
        "id": id,
        "appoint": data.appoint,
        "create_time": create_time,
        "comment": data.comment
    })))
}
#[derive(Deserialize)]
struct UpdateCommentParams {
    id: String,
    comment: String,
}

async fn update_comment(header: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: UpdateCommentParams = serde_json::from_value(value)?;
    conn.query_drop(format!(
        "UPDATE appoint_comment SET comment = '{}' WHERE id = '{}' AND applicant = '{uid}' LIMIT 1
    ",
        data.comment, data.id
    ))?;
    Ok(Response::empty())
}

async fn delete_comment(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    conn.query_drop(format!(
        "DELETE FROM appoint_comment WHERE id = '{id}' AND applicant = '{uid}' LIMIT 1"
    ))?;
    Ok(Response::empty())
}
