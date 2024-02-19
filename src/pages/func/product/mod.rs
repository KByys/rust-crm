use axum::{extract::Multipart, http::HeaderMap, Router};
use serde::Deserialize;

use crate::{bearer, database::get_conn, libs::parse_multipart, parse_jwt_macro, ResponseResult};

pub fn product_router() -> Router {
    Router::new()
}
#[derive(Debug, Deserialize)]
struct InsertParams {
    /// 编号
    number: String,
    name: String,
    /// 规格
    specifications: String,
    /// 型号
    model: String,
    /// 单位
    unit: String,
    /// 数量
    amount: String,
    product_type: String,
    price: f32,
    /// 条形码
    barcode: String,
    explanation: String,
    
    storehouse: String
}

async fn add_product(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let part = parse_multipart(part).await?;
    todo!()
}
