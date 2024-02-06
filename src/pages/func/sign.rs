use axum::{
    extract::{Multipart, Path},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
// use mysql_common::prelude::FromRow;
use op::ternary;
use rand::random;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;

use crate::{
    base64_encode, bearer,
    common::{empty_deserialize_to_none, Person},
    database::{c_or_r_more, get_conn},
    libs::{gen_id, parse_multipart, time::TIME, FilePart},
    pages::account::get_user,
    parse_jwt_macro,
    perm::verify_permissions,
    response::BodyFile,
    Response, ResponseResult,
};

#[derive(Deserialize, Serialize, Default, FromRow)]
pub struct SignRecord {
    #[serde(default)]
    id: String,
    #[serde(default)]
    signer: Person,
    #[serde(deserialize_with = "deserialize_time")]
    sign_time: String,
    address: String,
    location: String,
    #[serde(deserialize_with = "empty_deserialize_to_none")]
    customer: Option<Person>,
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
    sign.signer.phone = id;
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
    sign.signer.phone = id;
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
        "INSERT INTO sgin (id, signer, customer, address, sign_time, file, content)
        VALUES (:id, :signer, :customer, :address, :sign_time, :file, :content)",
        params! {
            "id" => &data.id,
            "signer" => &data.signer.phone(),
            "customer" => data.customer.as_ref().map_or("NULL", |c|c.phone()),
            "address" => &data.address,
            "location" => &data.location,
            "sign_time" => &data.sign_time,
            "file" => op::ternary!(files.is_empty() => "NULL"; &data.file),
            "content" => &data.content,
        },
    )?;
    for (f, path) in files {
        std::fs::write(format!("resourses/sign/{path}"), &f.bytes)?;
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

// async fn check(role: &str, data: Option<&[&str]>) -> Result<(), Response> {
//     let f = verify_permissions(role, "other", "query_sign_in", data).await;
//     ternary!(f => Ok(()); Err(Response::permission_denied()))
// }
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
    let mut records = match data.scope {
        // 个人签到记录
        0 => {
            if user.id == data.data {
                __query(&mut conn, Some(&id), &[user.department], &data.date_range)?
            } else if !data.data.is_empty() {
                let signer = get_user(&data.data, &mut conn)?;
                if user.department == signer.department {
                    check_perm!(&user.role, None)
                } else {
                    check_perm!(&user.role, Some(&[&"all"]))
                }
                __query(
                    &mut conn,
                    Some(&data.data),
                    &[signer.department],
                    &data.date_range,
                )?
            } else {
                return Err(Response::invalid_value("data为空字符串"));
            }
        }
        // 部门签到记录
        1 => {
            if user.department == data.data {
                check_perm!(&user.role, None)
            } else {
                check_perm!(&user.role, Some(&[&"all"]))
            }
            __query(&mut conn, None, &[user.department], &data.date_range)?
        }
        // 全公司签到记录
        2 => {
            check_perm!(&user.role, Some(&[&"all"]));
            let departs = conn.query_map(
                "SELECT * FROM department WHERE value != '总经办'",
                |f: String| f,
            )?;
            __query(&mut conn, None, &departs, &data.date_range)?
        }
        _ => return Err(Response::invalid_value("scope数值不对")),
    };
    complete_data(&mut records, &mut conn)?;
    Ok(Response::ok(json!(records)))
}
fn complete_data(
    data: &mut [(String, Vec<SignRecord>)],
    conn: &mut PooledConn,
) -> mysql::Result<()> {
    for (_, records) in data {
        for r in records {
            r.signer.name = conn
                .query_first(format!(
                    "SELECT name FROM user WHERE id = '{}'",
                    r.signer.phone
                ))?
                .unwrap_or_default();
            if let Some(c) = &mut r.customer {
                c.name = conn
                    .query_first(format!(
                        "SELECT name FROM customer WHERE id = '{}'",
                        r.signer.phone
                    ))?
                    .unwrap_or_default();
            }
        }
    }
    Ok(())
}
async fn get_file(Path(img): Path<String>) -> Result<BodyFile, (StatusCode, String)> {
    BodyFile::new_with_base64_url("resourses/sign", &img)
}
fn __query(
    conn: &mut PooledConn,
    signer: Option<&str>,
    depart: &[String],
    (start, end): &(String, String),
) -> Result<Vec<(String, Vec<SignRecord>)>, Response> {
    let mut records = Vec::new();
    if let Some(signer) = signer {
        let record = conn.query_map(
            format!(
                "SELECT *  FROM sign WHERE signer = '{signer}' 
                AND (sign_time >= '{}' AND sign_time <= '{}') ORDER BY sign_time",
                start, end
            ),
            |r| r,
        )?;
        records.push((depart[0].to_string(), record))
    } else {
        for d in depart {
            let record = conn.query_map(
                format!(
                    "SELECT DISTINCT s.* FROM sign s JOIN user u ON u.id = s.signer 
                        WHERE u.department = '{d}'AND (s.sign_time >= '{start}' AND s.sign_time <= '{end}') ORDER BY sign_time"
                ),
                |f| f,
            )?;
            records.push((d.to_string(), record))
        }
    }

    Ok(records)
}
