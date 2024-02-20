use crate::{
    bearer,
    database::{c_or_r, get_conn},
    libs::{gen_id, parse_multipart, FilePart, TIME},
    parse_jwt_macro, Response, ResponseResult,
};
use axum::{extract::Multipart, http::HeaderMap};
use mysql::{prelude::Queryable, PooledConn};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct ProductParams {
    #[serde(default)]
    id: String,
    #[serde(skip_deserializing)]
    create_time: String,
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
    storehouse: String,
}

async fn add_product(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let part = parse_multipart(part).await?;
    let data: ProductParams = serde_json::from_str(&part.json)?;
    let file = op::some!(part.files.first(); ret Err(Response::dissatisfy("缺少封面")));

    c_or_r(__insert, &mut conn, (data, file), false)?;
    Ok(Response::empty())
}

fn __insert(
    conn: &mut PooledConn,
    (mut data, part): (ProductParams, &FilePart),
) -> Result<(), Response> {
    let time = TIME::now()?;
    data.id = gen_id(&time, &data.name);
    let pinyin = rust_pinyin::get_pinyin(&data.name);
    let n: Option<i32> = conn.query_first(format!(
        "SELECT MAX(num) FROM product_num WHERE name='{}' GROUP BY name",
        pinyin
    ))?;
    let n = n.unwrap_or(0) + 1;
    if data.number.is_empty() {
        data.number = format!("NO.{}{:0>7}", pinyin, n)
    }
    conn.query_drop("INSERT INTO product_num (name, num) VALUES ('{pinyin}', {n})")?;
    
    todo!()
}
