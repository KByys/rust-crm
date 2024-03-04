use crate::{
    bearer,
    database::{c_or_r, get_conn},
    libs::{
        dser::{deser_f32, serialize_f32_to_string},
        gen_file_link, gen_id, parse_multipart, FilePart, TimeFormat, TIME,
    },
    pages::{
        account::get_user,
        func::{
            __insert_custom_fields, __update_custom_fields, customer::index::CustomCustomerData,
            get_custom_fields,
        },
        setting::option::check_drop_down_box,
    },
    parse_jwt_macro,
    perm::verify_permissions,
    response::BodyFile,
    Response, ResponseResult,
};
use axum::{
    extract::{Multipart, Path},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub fn product_router() -> Router {
    Router::new()
        .route("/product/add", post(add_product))
        .route("/product/update", post(update_product))
        .route("/product/update/json", post(update_product_json))
        .route("/product/delete/:id", delete(delete_product))
        .route("/product/app/list/data", post(query_product))
        .route("/product/query/:id", get(query_by))
        .route("/product/cover/:cover", get(get_cover))
}

#[derive(Debug, Deserialize, Serialize, mysql_common::prelude::FromRow)]
struct ProductParams {
    #[serde(default)]
    id: String,
    #[serde(skip_deserializing)]
    create_time: String,
    #[serde(default)]
    cover: String,
    /// 编号
    num: String,
    name: String,
    /// 规格
    specification: String,
    /// 型号
    model: String,
    /// 单位
    unit: String,
    /// 数量
    amount: usize,
    product_type: String,
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    price: f32,
    /// 条形码
    barcode: String,
    explanation: String,
    storehouse: String,
    custom_fields: CustomCustomerData,
}

async fn add_product(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn)?;
    if !verify_permissions(&user.role, "storehouse", "product", Some(&["create"])).await {
        return Err(Response::permission_denied());
    }
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
        "SELECT num FROM product_num WHERE name='{}'",
        pinyin
    ))?;

    let n = n.unwrap_or(0) + 1;
    if data.num.is_empty() {
        data.num = format!("NO.{}{:0>7}", pinyin, n)
    }
    if let Some(true) = check_drop_down_box("storehouse", &data.storehouse) {
        // nothing
    } else {
        return Err(Response::not_exist("库房不存在"));
    }
    conn.query_drop(format!(
        "INSERT INTO product_num (name, num) VALUES ('{pinyin}', {n})
    ON DUPLICATE KEY UPDATE num = {n}"
    ))?;
    data.create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    let link = gen_file_link(&time, part.filename());
    conn.exec_drop(
        "INSERT INTO product (id, num, name, specification, cover, model, unit,
        amount, product_type, price, create_time, barcode, explanation, storehouse) VALUES (
        :id, :num, :name, :specification, :cover, :model, :unit,
        :amount, :product_type, :price, :create_time, :barcode, :explanation, :storehouse
        )",
        params! {
            "id" => &data.id, "num" => data.num, "name" => data.name,
            "specification" => data.specification, "cover" => &link,
            "model" => data.model, "unit" => data.unit, "amount" => data.amount,
            "product_type" => data.product_type, "price" => data.price,
            "explanation" => data.explanation, "storehouse" => data.storehouse,
            "create_time" => data.create_time, "barcode" => data.barcode,

        },
    )?;
    __insert_custom_fields(conn, &data.custom_fields.inner, 1, &data.id)?;
    std::fs::write(format!("resources/product/cover/{link}"), &part.bytes)?;
    Ok(())
}

async fn update_product(header: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn)?;
    if !verify_permissions(&user.role, "storehouse", "product", Some(&["update"])).await {
        return Err(Response::permission_denied());
    }

    let part = parse_multipart(part).await?;
    let data: ProductParams = serde_json::from_str(&part.json)?;
    let file = op::some!(part.files.first(); ret Err(Response::dissatisfy("缺少封面")));
    c_or_r(__update, &mut conn, (data, Some(file)), false)?;
    Ok(Response::empty())
}

async fn update_product_json(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&id, &mut conn)?;
    if !verify_permissions(&user.role, "storehouse", "product", Some(&["update"])).await {
        return Err(Response::permission_denied());
    }
    let data: ProductParams = serde_json::from_value(value)?;
    c_or_r(__update, &mut conn, (data, None), false)?;
    Ok(Response::empty())
}

fn __update(
    conn: &mut PooledConn,
    (data, part): (ProductParams, Option<&FilePart>),
) -> Result<(), Response> {
    if let Some(true) = check_drop_down_box("storehouse", &data.storehouse) {
        // nothing
    } else {
        return Err(Response::not_exist("库房不存在"));
    }
    let cover: Option<String> = conn.query_first(format!(
        "SELECT cover FROM product WHERE id = '{}' LIMIT 1",
        data.id
    ))?;
    println!("cover is --{:?}", cover);
    let cover = op::some!(cover; ret Err(Response::not_exist("code: 180909")));
    let time = TIME::now()?;

    let link = if let Some(f) = part {
        println!("{}", f.filename());
        let link = gen_file_link(&time, f.filename());
        link
    } else {
        cover.clone()
    };

    println!("{:#?}", data);
    conn.exec_drop(format!("UPDATE product SET num=:num, name=:name, specification=:specification,
        cover=:cover, model=:model, unit=:unit, amount=:amount, product_type=:product_type, price=:price,
        barcode=:barcode, explanation=:explanation, storehouse=:storehouse WHERE id = '{}' LIMIT 1", data.id), 
        params! {
            "num" => data.num, "name" => data.name,
            "specification" => data.specification, "cover" => &link,
            "model" => data.model, "unit" => data.unit, "amount" => data.amount,
            "product_type" => data.product_type, "price" => data.price,
            "explanation" => &data.explanation, "storehouse" => &data.storehouse,
            "barcode" => data.barcode,
        }
    )?;

    __update_custom_fields(conn, &data.custom_fields.inner, 1, &data.id)?;
    if let Some(f) = part {
        std::fs::write(format!("resources/product/cover/{link}"), &f.bytes)?;
        println!("remove -- {}", cover);
        std::fs::remove_file(format!("resources/product/cover/{cover}"))?;
    }
    Ok(())
}
#[derive(Debug, Deserialize)]
struct QueryParams {
    stock: usize,
    ty: String,
    storehouse: String,
}

async fn query_product(Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let data: QueryParams = serde_json::from_value(value)?;
    println!("{:#?}", data);
    let stock = match data.stock {
        1 => "> 0",
        2 => "= 0",
        _ => ">= 0",
    };
    let ty = op::ternary!(data.ty.is_empty() => "IS NOT NULL".into(); format!("= '{}'", data.ty));
    let storehouse = op::ternary!(data.storehouse.is_empty() => ">''".into(); format!("= '{}'", data.storehouse));
    println!(
         "SELECT *, 1 as custom_fields FROM product WHERE product_type {ty} AND storehouse {storehouse} AND amount {stock}"
    );
    let mut products: Vec<ProductParams> = conn.query(format!(
        "SELECT *, 1 as custom_fields FROM product WHERE product_type {ty} AND storehouse {storehouse} AND amount {stock}"))?;
    for product in &mut products {
        product.custom_fields = get_custom_fields(&mut conn, &product.id, 1)?;
    }
    println!("{:#?}", products);
    Ok(Response::ok(json!(products)))
}

async fn query_by(Path(id): Path<String>) -> ResponseResult {
    let mut conn = get_conn()?;
    let mut data: Option<ProductParams> = conn.query_first(format!(
        "SELECT *, 1 as custom_fields FROM product WHERE id = '{id}' ORDER BY create_time"
    ))?;
    if let Some(d) = &mut data {
        d.custom_fields = get_custom_fields(&mut conn, &d.id, 1)?;
    }
    Ok(Response::ok(json!(data)))
}

async fn delete_product(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let user = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&user, &mut conn)?;
    if !verify_permissions(&user.role, "storehouse", "product", Some(&["delete"])).await {
        return Err(Response::permission_denied());
    }
    c_or_r(__delete_product, &mut conn, &id, false)?;
    Ok(Response::empty())
}
fn __delete_product(conn: &mut PooledConn, id: &str) -> Result<(), Response> {
    let cover: Option<String> = conn.query_first(format!("select cover from product where id = '{id}'"))?;
    conn.query_drop(format!("DELETE FROM custom_field_data WHERE id = '{id}'"))?;
    conn.query_drop(format!("DELETE FROM product WHERE id = '{id}' LIMIT 1"))?;
    if let Some(cover) = cover {
        std::fs::remove_file(format!("resources/product/cover/{cover}"))?;
    }
    Ok(())
}

async fn get_cover(Path(cover): Path<String>) -> Result<BodyFile, (StatusCode, String)> {
    BodyFile::new_with_base64_url("resources/product/cover", &cover)
}
