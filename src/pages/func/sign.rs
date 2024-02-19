use axum::{
    extract::{Multipart, Path},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use op::ternary;
use rand::random;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;

use crate::{
    base64_encode, bearer,
    common::empty_deserialize_to_none,
    database::{c_or_r_more, get_conn},
    libs::{gen_id, parse_multipart, time::TIME, FilePart},
    pages::account::get_user,
    parse_jwt_macro,
    perm::verify_permissions,
    response::BodyFile,
    Response, ResponseResult, ID,
};

#[derive(Deserialize, Serialize, Default, FromRow)]
pub struct SignRecord {
    #[serde(default)]
    id: String,
    #[serde(default)]
    signer: String,
    #[serde(skip_deserializing)]
    signer_name: String,
    #[serde(deserialize_with = "deserialize_time")]
    sign_time: String,
    address: String,
    location: String,
    #[serde(deserialize_with = "empty_deserialize_to_none")]
    customer: Option<String>,
    #[serde(skip_deserializing)]
    customer_name: Option<String>,
    #[serde(skip_deserializing)]
    file: String,
    content: String,
}
pub fn sign_router() -> Router {
    Router::new()
        .route("/sign/in", post(sign))
        .route("/sign/in/json", post(sign_only_json))
        .route("/sign/records", post(query_sign_records))
        .route("/sign/img/:img", get(get_file))
        .route("/sign/delete", delete(delete_sign))
}
pub fn deserialize_time<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let regex = Regex::new(r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})").unwrap();
    let s = String::deserialize(de)?;
    if regex.is_match(&s) {
        Ok(s)
    } else {
        Err(serde::de::Error::custom(
            "Invalid Time Format. 时间格式应当为'YYYY-MM-DD HH:MM'",
        ))
    }
}
async fn sign_only_json(header: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let time = TIME::now()?;
    let mut sign: SignRecord = serde_json::from_value(value)?;
    sign.id = gen_id(&time, &base64_encode(random::<i32>().to_string()));
    sign.signer = id;
    conn.query_drop("BEGIN")?;
    c_or_r_more(_insert, &mut conn, &sign, &[])?;
    Ok(Response::empty())
}

async fn sign(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data = parse_multipart(part).await?;
    let time = TIME::now()?;
    let mut sign: SignRecord = serde_json::from_str(&data.json)?;
    sign.id = gen_id(&time, &base64_encode(random::<i32>().to_string()));
    sign.signer = id;
    let mut files = Vec::new();
    for part in &data.files {
        let id = gen_id(&time, part.filename.as_deref().unwrap_or("unknown.jpg"));
        sign.file.push_str(&format!("{}&", id));
        files.push((part, id));
    }
    sign.file.pop();
    conn.query_drop("BEGIN")?;
    c_or_r_more(_insert, &mut conn, &sign, &files)?;
    todo!()
}
fn _insert(
    conn: &mut PooledConn,
    data: &SignRecord,
    files: &[(&FilePart, String)],
) -> Result<(), Response> {
    conn.exec_drop(
        "INSERT INTO sign (id, signer, customer, address, sign_time, file, content)
        VALUES (:id, :signer, :customer, :address, :sign_time, :file, :content)",
        params! {
            "id" => &data.id,
            "signer" => &data.signer,
            "customer" => &data.customer,
            "address" => &data.address,
            "location" => &data.location,
            "sign_time" => &data.sign_time,
            "file" => op::ternary!(files.is_empty() => "NULL"; &data.file),
            "content" => &data.content,
        },
    )?;
    for (f, path) in files {
        std::fs::write(format!("resources/sign/{path}"), &f.bytes)?;
    }
    Ok(())
}
#[derive(serde::Deserialize)]
struct Record {
    #[serde(deserialize_with = "deserialize_date_range")]
    date_range: (String, String),
    scope: i32,
    data: String,
}
fn deserialize_date_range<'de, D>(deser: D) -> Result<(String, String), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let range = String::deserialize(deser)?;
    let split_range: Vec<_> = range.splitn(2, '~').collect();
    let regex = Regex::new(r"(\d{4})-(\d{2})-(\d{2})").unwrap();
    let captures = |i| regex.captures(split_range[i]).map(|s| s.extract::<3>().0);
    let start = captures(0);
    let end = captures(1);
    Ok((
        start.unwrap_or("0000-00-00").to_owned(),
        end.unwrap_or("3000-00-00").to_owned(),
    ))
}

macro_rules! check_perm {
    ($role:expr, $data:expr) => { {
        let f = verify_permissions($role, "other", "query_sign_in", $data).await;
        ternary!(f => (); return Err(Response::permission_denied()))

    }
    };
}

async fn query_sign_records(
    header: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: Record = serde_json::from_value(value)?;
    let user = get_user(&id, &mut conn)?;
    let records = match data.scope {
        // 个人签到记录
        0 => {
            if user.id == data.data || data.data.is_empty() {
                select_sign(&user.id, &mut conn)?
            } else  {
                let signer = get_user(&data.data, &mut conn)?;
                if user.department == signer.department {
                    check_perm!(&user.role, None)
                } else {
                    check_perm!(&user.role, Some(&[&"all"]))
                }
                select_sign(&data.data, &mut conn)?
            }
        }
        // 部门签到记录
        1 => {
            if user.department == data.data {
                check_perm!(&user.role, None)
            } else {
                check_perm!(&user.role, Some(&[&"all"]))
            }
            select_sign_with_depart(&data, &mut conn, &data.data)?
        }
        // 全公司签到记录
        2 => {
            check_perm!(&user.role, Some(&[&"all"]));
            select_company_signs(&data, &mut conn)?
        }
        _ => return Err(Response::invalid_value("scope数值不对")),
    };
    Ok(Response::ok(json!(records)))
}

fn select_sign(signer: &str, conn: &mut PooledConn) -> mysql::Result<Vec<SignRecord>> {
    conn.query_map(
        format!(
            "SELECT DISTINCT s.*, u.name as signer_name, c.name as customer_name 
            FROM sign s 
            JOIN user ON u.id = s.signer
            LEFT JOIN customer c ON c.id = s.customer
            WHERE s.signer = '{signer}'"
        ),
        |r| r,
    )
}
fn select_sign_with_depart(
    r: &Record,
    conn: &mut PooledConn,
    depart: &str,
) -> mysql::Result<Vec<SignRecord>> {
    conn.query_map(
        format!(
            "SELECT s.*, u.name as signer_name, c.name as customer_name FROM sign s
        JOIN user u ON u.department='{depart}' AND u.id = s.signer
        LEFT JOIN customer c ON c.id = s.customer
        WHERE sign_time >= '{}' AND sign_time <= '{}'
        ORDER BY DESC s.sign_time",
            r.date_range.0, r.date_range.1
        ),
        |r| r,
    )
}

fn select_company_signs(r: &Record, conn: &mut PooledConn) -> mysql::Result<Vec<SignRecord>> {
    conn.query_map(
        format!(
            "SELECT s.*, u.name as signer_name, c.name as customer_name FROM sign s
        LEFT JOIN customer c ON c.id = s.customer
        JOIN user u ON u.id = s.signer
        WHERE s.sign_time >= '{}' AND s.sign_time <= '{}'
        ORDER BY DESC s.sign_time",
            r.date_range.0, r.date_range.1
        ),
        |r| r,
    )
}
async fn get_file(Path(img): Path<String>) -> Result<BodyFile, (StatusCode, String)> {
    BodyFile::new_with_base64_url("resourses/sign", &img)
}

async fn delete_sign(header: HeaderMap, Json(value): Json<serde_json::Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let sign_id: ID = serde_json::from_value(value)?;
    conn.query_drop(format!("DELETE sign WHERE id = '{}' AND signer = '{id}' LIMIT 1", sign_id.id))?;
    Ok(Response::empty())
}

