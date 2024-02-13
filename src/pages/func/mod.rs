mod product;
mod report;
mod sea;
mod sign;
use std::collections::HashMap;

use axum::Router;
use mysql::prelude::Queryable;

mod customer;

pub fn func_router() -> Router {
    customer::customer_router()
        .merge(sea::sea_router())
        .merge(product::product_router())
        .merge(report::report_router())
        .merge(sign::sign_router())
}

pub fn verify_custom_fields(ver: &[&str], data: &[crate::Field]) -> bool {
    ver.len() == data.len() && {
        data.iter()
            .all(|info| ver.iter().all(|v| info.display.eq(v)))
    }
}

pub fn __insert_custom_fields(
    conn: &mut mysql::PooledConn,
    fields: &HashMap<String, Vec<crate::Field>>,
    ty: u8, id: &str
) -> Result<(), crate::Response> {
    let (texts, times, boxes) = unsafe { crate::pages::STATIC_CUSTOM_FIELDS.get_fields(0) };

    let map: HashMap<&str, Vec<&str>> = [("texts", texts), ("times", times), ("boxes", boxes)]
        .into_iter()
        .collect();
    for (k, v) in &map {
        if let Some(d) = fields.get(*k) {
            if !verify_custom_fields(v, d) {
                return Err(crate::Response::dissatisfy("自定义字段存在不匹配情况"));
            }
        } else {
            return Err(crate::Response::dissatisfy("自定义字段存在不匹配情况"));
        }
    }
    let mut values = String::new();
    for (k, v) in fields {
        let s = op::some!(get_ty(k); con);
        for field in v {
            values.push_str(&format!("({ty}, {s}, '{id}', '{}', '{}'),",field.display, field.value ));
        }
    }
    values.pop();
    if !values.is_empty() {
        conn.query_drop(format!("INSERT INTO custom_field_data (fields, ty, id, display, value) VALUES {values}"))?;
    }

    Ok(())
}

fn get_ty(s: &str) -> Option<i32> {
    match s {
        "texts" => Some(0),
        "times" => Some(1),
        "boxes" => Some(2),
        _ => None 
    }
}