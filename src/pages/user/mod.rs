use axum::{extract::Path, routing::post, Router};
use mysql::prelude::Queryable;
use serde_json::json;

use crate::{database::get_conn, Response, ResponseResult};

pub fn user_router() -> Router {
    Router::new().route("/user/name/:id", post(get_user_name))
}

async fn get_user_name(Path(id): Path<String>) -> ResponseResult {
    let mut conn = get_conn()?;
    let name: Option<String> = conn.query_first(format!("SELECT name FROM user WHERE id = '{id}' LIMIT 1"))?;
    Ok(Response::ok(json!(name)))
}
