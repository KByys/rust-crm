use std::collections::HashMap;

use axum::{
    extract::{Multipart, Path},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use op::ternary;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback,
    database::get_db,
    libs::{
        dser::{deser_empty_to_none, split_files},
        gen_file_link, gen_id, parse_multipart, FilePart, TimeFormat, TIME,
    },
    pages::account::get_user,
    parse_jwt_macro,
    perm::action::OtherGroup,
    response::BodyFile,
    verify_perms, Response, ResponseResult,
};

pub fn sign_router() -> Router {
    Router::new()
        .route("/sign/in", post(add_sign))
        .route("/sign/in/json", post(add_sign_json))
        .route("/sign/records", post(query_sign_records))
        .route("/sign/delete/:id", delete(delete_sign_record))
        .route("/sign/img/:id", get(sign_img))
}

#[derive(Debug, Deserialize)]
struct InsertParams {
    #[serde(default)]
    id: String,
    #[serde(default)]
    file: String,
    address: String,
    location: String,
    #[serde(deserialize_with = "deser_empty_to_none")]
    customer: Option<String>,
    #[serde(deserialize_with = "deser_empty_to_none")]
    appoint: Option<String>,
    content: String,
}

async fn add_sign(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let part = parse_multipart(part).await?;
    let param = serde_json::from_str(&part.json)?;
    let (id, file) = commit_or_rollback!(__add_sign, &mut conn, (&uid, &param, Some(&part.files)))?;
    Ok(Response::ok(json!({
        "id": id,
        "file": file
    })))
}

async fn add_sign_json(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let param: InsertParams = serde_json::from_value(value)?;
    commit_or_rollback!(__add_sign, &mut conn, (&uid, &param, None))?;
    Ok(Response::empty())
}
fn recv_mul_image(
    file: Option<&[FilePart]>,
    time: &TIME,
) -> Result<Option<String>, Response> {
    if let Some(f) = op::catch!(file?.first()) {
        let file_link = gen_file_link(time, f.filename());
        let parent = "resources/sign";
        std::fs::write(format!("{parent}/{file_link}"), &f.bytes)?;
        return Ok(Some(file_link));
    }
    Ok(None)
}
fn __add_sign(
    conn: &mut PooledConn,
    (uid, param, file): (&str, &InsertParams, Option<&[FilePart]>),
) -> Result<(String, Option<String>), Response> {
    let time = TIME::now()?;
    if !param.id.is_empty() && !param.file.is_empty() {
        let link = recv_mul_image(file, &time)?;
        if let Some(link) = link {
            let file_link = format!("{}&{}", param.file, link);
            conn.query_drop(format!(
                "update sign set file = '{file_link}' where id = '{}' limit 1",
                param.id
            ))?;
            return Ok((param.id.clone(), Some(file_link)));
        }
        return Ok(Default::default());
    }
    let file_link = match file {
        Some(files) => {
            let mut link = String::new();
            for f in files {
                link.push_str(&format!("{}&", gen_file_link(&time, f.filename())))
            }
            link.pop();
            Some(link)
        }
        None => None,
    };
    let id = gen_id(&time, "sign");
    conn.exec_drop(
        format!(
            "insert into sign 
            (id, signer, customer, address, appoint, location, sign_time, file, content)
            values ('{id}', '{}', :customer, '{}', :appoint, '{}', '{}', :file, '{}'
        )
    ",
            uid,
            param.address,
            param.location,
            time.format(TimeFormat::YYYYMMDD_HHMMSS),
            param.content
        ),
        params! {
            "customer" => &param.customer,
            "appoint" => &param.appoint,
            "file" => &file_link
        },
    )?;
    if let Some(files) = file {
        let links: Vec<_> = file_link.as_ref().expect("unreadable").split('&').collect();
        let parent = "resources/sign";
        for (i, f) in files.iter().enumerate() {
            std::fs::write(format!("{parent}/{}", links[i]), &f.bytes)?;
        }
    }
    Ok((id, file_link))
}
#[derive(Debug, Deserialize)]
struct QueryParams {
    start: String,
    end: String,
    scope: u8,
    data: String,
}

#[derive(Serialize, FromRow)]
struct SignRecord {
    id: String,
    signer: String,
    signer_name: String,
    address: String,
    location: String,
    sign_time: String,
    company: Option<String>,
    customer: Option<String>,
    customer_name: Option<String>,
    appoint: Option<String>,
    #[serde(skip_serializing)]
    department: String,
    #[serde(serialize_with = "split_files")]
    #[serde(rename = "files")]
    file: Option<String>,
    content: String,
}

async fn query_sign_records(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let param: QueryParams = serde_json::from_value(value)?;
    let end = ternary!(param.end.is_empty() => "9999-99-99 99:99:99".into(), format!("{} 99:99:99", param.end));
    match param.scope {
        0 => {
            let (id, depart) = if param.data.eq("my") || param.data.eq(&uid) {
                (uid.clone(), user.department)
            } else if let (true, all) = verify_perms!(
                &user.role,
                OtherGroup::NAME,
                OtherGroup::QUERY_SIGN_IN,
                None,
                Some(["all"].as_slice())
            ) {
                let other = get_user(&param.data, &mut conn).await?;
                if all || other.department.eq(&user.department) {
                    (other.id, other.department)
                } else {
                    return Err(Response::permission_denied());
                }
            } else {
                return Err(Response::permission_denied());
            };
            let query = format!(
                "select s.*, sr.name as signer_name, c.name as customer_name, c.company, 1 as department
                from sign s
                join user sr on sr.id = s.signer
                left join customer c on c.id = s.customer
                where signer = '{id}' and s.sign_time >= '{}' and s.sign_time <= '{end}' order by sign_time desc"
            , param.start);
            println!("{}", query);
            let records: Vec<SignRecord> = conn.query(query)?;
            Ok(Response::ok(json!([json!({
                "department": depart,
                "data": records
            })])))
        }
        1 => {
            let flag = {
                let (depart, all) = verify_perms!(
                    &user.role,
                    OtherGroup::NAME,
                    OtherGroup::QUERY_SIGN_IN,
                    None,
                    Some(["all"].as_slice())
                );
                all || (depart && param.data.eq("my") || param.data.eq(&user.department))
            };
            if flag {
                let depart = ternary!(param.data.eq("my") => user.department, param.data);

                let query = format!(
                    "select s.*, sr.name as signer_name, c.name as customer_name, c.company, 1 as department
                from sign s
                join user sr on sr.id = s.signer and sr.department = '{depart}'
                left join customer c on c.id = s.customer
                where s.sign_time >= '{}' and s.sign_time <= '{end}' order by sign_time desc",
                    param.start
                );
                println!("{}", query);
                let records: Vec<SignRecord> = conn.query(query)?;
                Ok(Response::ok(json!([json!({
                    "department": depart,
                    "data": records
                })])))
            } else {
                Err(Response::permission_denied())
            }
        }
        2 => {
            if verify_perms!(
                &user.role,
                OtherGroup::NAME,
                OtherGroup::QUERY_SIGN_IN,
                Some(["all"].as_slice())
            ) {
                let records: Vec<SignRecord> = conn.query(format!(
                    "select s.*, sr.name as signer_name, c.name as customer_name, c.company, sr.department as department
                from sign s
                join user sr on sr.id = s.signer
                left join customer c on c.id = s.customer
                where s.sign_time >= '{}' and s.sign_time <= '{end}' order by sign_time desc",
                    param.start
                ))?;
                let mut map: HashMap<String, Vec<SignRecord>> = HashMap::new();
                for record in records {
                    map.entry(record.department.clone())
                        .or_default()
                        .push(record)
                }
                let data: Vec<Value> = map
                    .into_iter()
                    .map(|(k, v)| {
                        json!({
                            "department": k,
                            "data": v
                        })
                    })
                    .collect();
                Ok(Response::ok(json!(data)))
            } else {
                Err(Response::permission_denied())
            }
        }
        _ => Err(Response::invalid_value("scope错误")),
    }
}

async fn delete_sign_record(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
        let db = get_db().await?;
    let mut conn = db.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let key: Option<(i32, Option<String>)> = conn.query_first(format!(
        "select 1, file from sign where id = '{id}' and signer = '{uid}' limit 1"
    ))?;
    if let Some((_, f)) = key {
        if let Some(file) = f {
            commit_or_rollback!(__delete_sign_record, &mut conn, (&id, &file))?;
        } else {
            conn.query_drop(format!("delete from sign where id = '{id}' limit 1"))?;
        }
        Ok(Response::empty())
    } else {
        Err(Response::permission_denied())
    }
}

fn __delete_sign_record(
    conn: &mut PooledConn,
    (id, file): (&str, &str),
) -> Result<(), Response> {
    conn.query_drop(format!("delete from sign where id = '{id}' limit 1"))?;
    for f in file.split('&') {
        ternary!(f.is_empty() => continue, ());
        std::fs::remove_file(format!("resources/sign/{f}"))?;
    }
    Ok(())
}

async fn sign_img(Path(id): Path<String>) -> Result<BodyFile, Response> {
    BodyFile::new_with_base64_url("resources/sign", &id)
        .map_err(|(code, msg)| Response::new(code, -1, json!(msg)))
}
