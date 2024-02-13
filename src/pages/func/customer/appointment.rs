
use axum::extract::Path;
use axum::routing::{delete, post};
use axum::{http::HeaderMap, Json, Router};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::libs::dser::deser_yyyy_mm_dd_hh_mm;
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
     .route("/customer/appointment/delete/:id", delete(delete_appointment))   
     .route("/customer/appointment/finish/:id", post(finish_appointment))   
     .route("/customer/appointment/data/:id", post(query_appointment))   
}
#[derive(Debug, Deserialize)]
struct InsertParams {
    customer: String,
    #[serde(deserialize_with = "deser_yyyy_mm_dd_hh_mm")]
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
            (id, customer, salesman, appointment, finish_time, theme, content) VALUES (
                '{}', '{}', '{}', '{}', NULL, '{}', '{}'
            )",
            id,
            param.customer,
            user_id,
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
    conn.query_drop(format!(
        "UPDATE appointment SET finish_time = '{}' WHERE id = '{}' LIMIT 1",
        time.format(TimeFormat::YYYYMMDD_HHMM) , id
    ))?;
    Ok(Response::empty())
}
#[derive(Debug, Serialize, FromRow)]
struct AppointmentResponse {
    id: String,
    salesman: String,
    appointment: String,
    finish_time: Option<String>,
    theme: String,
    content: String,
}

async fn query_appointment(Path(id): Path<String>) -> ResponseResult {
    let mut conn = get_conn()?;
    let res: Vec<AppointmentResponse> = conn.query(format!("SELECT * FROM appointment WHERE customer = '{}' ORDER BY appointment DESC", id))?;
    Ok(Response::ok(json!(res)))
}
