use axum::{
    extract::{Multipart, Path},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde_json::json;

use crate::{
    base64_encode, bearer,
    database::{c_or_r, c_or_r_more, get_conn},
    debug_info, do_if,
    libs::{
        parse_multipart,
        perm::Identity,
        time::{TimeFormat, TIME},
        FilePart,
    },
    pages::CUSTOM_FIELD_INFOS,
    parse_jwt_macro,
    response::BodyFile,
    Response, ResponseResult, TextInfos, ID,
};

use super::customer::CCInfos;
#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct Product {
    base_infos: BaseInfos,
    custom_infos: CCInfos,
}

#[derive(serde::Deserialize, serde::Serialize, FromRow, Debug)]
struct BaseInfos {
    /// 唯一标识
    #[serde(default)]
    id: String,
    /// 编号
    num: String,
    name: String,
    /// 封面
    cover: Option<String>,
    /// 规格
    specification: String,
    /// 型号
    model: String,
    unit: String,
    /// 数量
    amount: u64,
    #[serde(default)]
    create_time: String,
    product_type: String,
    price: f64,
    /// 条形码
    barcode: String,
    /// 说明
    explanation: String,
    /// 仓库
    storehouse: String,
}

pub fn product_router() -> Router {
    Router::new()
        .route("/product/add", post(add_product))
        .route("/product/update", post(update_product))
        .route("/products", post(query_product_infos))
        .route("/product/cover/:cover", get(get_product_cover))
        .route("/product/delete", delete(delete_product))
}

fn gen_product_num(name: &str, conn: &mut PooledConn) -> mysql::Result<String> {
    let pinyin = rust_pinyin::get_pinyin(name);
    let num: Option<u64> = conn.query_first(format!(
        "SELECT num FROM product_num WHERE name = '{}'",
        pinyin
    ))?;
    if let Some(n) = num {
        conn.query_drop(format!(
            "UPDATE product_num SET num = {} WHERE name = '{}'",
            n + 2,
            name
        ))?;
        Ok(format!("NO.{}{:6>0}", name, n + 1))
    } else {
        conn.query_drop(format!(
            "INSERT INTO product_num (name, num) VALUES ('{pinyin}', 1)"
        ))?;
        Ok(format!("NO.{}000001", pinyin))
    }
}

async fn add_product(headers: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data = parse_multipart(part).await?;
    let mut product: Product = serde_json::from_str(&data.json)?;
    debug_info(format!(
        "用户 {id} 执行添加产品操作，具体数据为{:?}",
        product
    ));

    if product.base_infos.num.is_empty() {
        product.base_infos.num = gen_product_num(&product.base_infos.name, &mut conn)?;
    }
    let time = TIME::now()?;
    product.base_infos.id = base64_encode(format!(
        "{}-{}-{}",
        product.base_infos.name,
        time.naos() / 10000,
        rand::random::<u8>()
    ));
    product.base_infos.create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    product.base_infos.cover = if let Some(file) = data.files.first() {
        let filename = file.filename.as_deref().unwrap_or("unknown.jpg");
        let base64_path = base64_encode(filename);
        Some(base64_path)
    } else {
        None
    };
    conn.query_drop("BEGIN")?;
    c_or_r_more(_insert, &mut conn, &product, data.files.first())?;
    Ok(Response::ok(json!({
        "id": product.base_infos.id
    })))
}

fn _insert(
    conn: &mut PooledConn,
    product: &Product,
    more: Option<&FilePart>,
) -> Result<(), Response> {
    let base_infos = &product.base_infos;
    let custom_infos = &product.custom_infos;
    println!("{:#?}", base_infos);
    conn.exec_drop("INSERT INTO product 
        (id, num, name, specification, cover, model, unit, amount, product_type, price, create_time, barcode, explanation, storehouse) 
        VALUES (
        :id, :num, :name, :specification, :cover, :model, :unit, :amount, :product_type, :price, :create_time, :barcode, :explanation, :storehouse
    )", params! {
        "id" => &base_infos.id,
        "num" => &base_infos.num,
        "name" => &base_infos.name,
        "specification" => &base_infos.specification,
        "cover" => base_infos.cover.as_deref().unwrap_or("NULL"),
        "model" => &base_infos.model,
        "unit" => &base_infos.unit,
        "amount" => &base_infos.amount,
        "product_type" => &base_infos.product_type,
        "price" => base_infos.price,
        "create_time" => &base_infos.create_time,
        "barcode" => &base_infos.barcode,
        "explanation" => &base_infos.explanation,
        "storehouse" => &base_infos.storehouse,
    })?;
    for i in 0..3 {
        let table = CUSTOM_FIELD_INFOS[1][i];
        if custom_infos.get(i).is_empty() {
            continue;
        }
        conn.query_drop(format!(
            "INSERT INTO {table} (id, display, value) VALUES {}",
            custom_infos.generate_sql(i, &base_infos.id)
        ))?;
    }
    if let Some(path) = &base_infos.cover {
        if let Some(f) = more {
            std::fs::write(format!("resources/product/{path}"), &f.bytes)?;
        }
    }
    Ok(())
}

async fn update_product(headers: HeaderMap, part: Multipart) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data = parse_multipart(part).await?;
    let mut product: Product = serde_json::from_str(&data.json)?;
    debug_info(format!(
        "用户 {id} 执行更新产品操作，具体数据为{:?}",
        product
    ));

    if product.base_infos.num.is_empty() {
        product.base_infos.num = gen_product_num(&product.base_infos.name, &mut conn)?;
    }
    let time = TIME::now()?;
    product.base_infos.id = base64_encode(format!(
        "{}-{}-{}",
        product.base_infos.name,
        time.naos() / 10000,
        rand::random::<u8>()
    ));
    product.base_infos.create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    let cover = product.base_infos.cover.clone();
    product.base_infos.cover = if let Some(file) = data.files.first() {
        let filename = file.filename.as_deref().unwrap_or("unknown.jpg");
        let base64_path = base64_encode(filename);
        Some(base64_path)
    } else {
        product.base_infos.cover
    };
    conn.query_drop("BEGIN")?;
    c_or_r_more(_update, &mut conn, &product, (data.files.first(), cover))?;
    Ok(Response::ok(json!({
        "id": product.base_infos.id
    })))
}

fn _update(
    conn: &mut PooledConn,
    product: &Product,
    more: (Option<&FilePart>, Option<String>),
) -> Result<(), Response> {
    let base_infos = &product.base_infos;
    let custom_infos = &product.custom_infos;
    conn.exec_drop(format!("UPDATE product SET
        num = :num, name = :name, specification=:specification, cover=:cover, model=:model, unit=:unit, 
        amount=:amount, product_type=:product_type, price=:price, barcode=:barcode, explanation=:explanation, storehouse=:storehouse
        WHERE id = '{}'
    ", base_infos.id), params! {
        "num" => &base_infos.num,
        "name" => &base_infos.name,
        "specification" => &base_infos.specification,
        "cover" => base_infos.cover.as_deref().unwrap_or("NULL"),
        "model" => &base_infos.model,
        "unit" => &base_infos.unit,
        "amount" => &base_infos.amount,
        "product_type" => &base_infos.product_type,
        "price" => base_infos.price,
        "barcode" => &base_infos.barcode,
        "explanation" => &base_infos.explanation,
        "storehouse" => &base_infos.storehouse,
    })?;
    for i in 0..3 {
        let table = CUSTOM_FIELD_INFOS[0][i];
        // 修改自定义字段
        for values in custom_infos.get(i) {
            conn.query_drop(format!(
                "UPDATE {table} SET value = '{}' WHERE id = '{}' AND display = '{}'",
                values.value, base_infos.id, values.display
            ))?;
        }
    }
    if let Some(path) = &base_infos.cover {
        if let Some(f) = more.0 {
            std::fs::write(format!("resources/product/{path}"), &f.bytes)?;
            // 如果之前存在的封面就删除
            if let Some(path) = more.1 {
                std::fs::remove_file(format!("resources/product/{path}"))?;
            }
        }
    }
    Ok(())
}
#[derive(serde::Deserialize)]
struct QueryProductParams {
    stock: usize,
    ty: String,
    sort: usize,
    storehouse: String,
}

async fn get_product_cover(Path(cover): Path<String>) -> Result<BodyFile, (StatusCode, String)> {
    BodyFile::new_with_base64_url("resources/product", &cover)
}
async fn query_product_infos(Json(value): Json<serde_json::Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let data: QueryProductParams = serde_json::from_value(value)?;
    let stock = match data.stock {
        0 => "",
        1 => "amount > 0",
        2 => "amount = 0",
        _ => return Err(Response::invalid_value("stock的值只能是0 1 2")),
    };
    let f = if data.ty.is_empty() {
        stock.to_owned()
    } else if stock.is_empty() {
        format!("product_type = '{}'", data.ty)
    } else {
        format!("product_type = '{}' AND {stock}", data.ty)
    };
    let f = do_if!(data.storehouse.is_empty() => f,  
        do_if!(f.is_empty() => format!("WHERE storehouse = '{}'", data.storehouse), 
            format!("WHERE {f} AND storehouse = '{}'", data.storehouse)));
    let mut base_infos: Vec<BaseInfos> =
        conn.query_map(format!("SELECT * FROM product {f}"), |d| d)?;
    let sort = match data.sort {
        0 => |v1: &BaseInfos, v2: &BaseInfos| {
            rust_pinyin::get_pinyin(&v1.name).cmp(&rust_pinyin::get_pinyin(&v2.name))
        },
        1 => |v1: &BaseInfos, v2: &BaseInfos| v1.num.cmp(&v2.num),

        2 => |v1: &BaseInfos, v2: &BaseInfos| v1.create_time.cmp(&v2.create_time),
        _ => |_v1: &BaseInfos, _v2: &BaseInfos| std::cmp::Ordering::Equal,
    };
    base_infos.sort_by(sort);
    let mut products = Vec::new();
    for info in base_infos {
        let mut custom_infos = CCInfos::default();
        for i in 0..=2 {
            let infos: Vec<TextInfos> = conn.query_map(
                format!(
                    "SELECT display, value FROM {} WHERE id = '{}'",
                    CUSTOM_FIELD_INFOS[1][i], info.id
                ),
                |info| info,
            )?;
            *custom_infos.get_mut(i) = infos;
        }
        products.push(Product {
            base_infos: info,
            custom_infos,
        });
    }
    Ok(Response::ok(json!(products)))
}

async fn delete_product(
    headers: HeaderMap,
    Json(value): Json<serde_json::Value>,
) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    if let Identity::Staff(_, _) = Identity::new(&id, &mut conn)? {
        return Err(Response::permission_denied());
    };
    let product_id: ID = serde_json::from_value(value)?;
    let product: Option<BaseInfos> = conn.query_first(format!(
        "SELECT * FROM product WHERE id = '{}'",
        product_id.id
    ))?;
    conn.query_drop("BEGIN")?;
    match product {
        Some(product) => {
            debug_info(format!(
                "用户 {} 进行删除产品操作，产品名：{}，编号：{}，",
                id, product.name, product.num
            ));
            c_or_r(_delete, &mut conn, &product, false)?;
            Ok(Response::empty())
        }
        _ => Err(Response::not_exist("该产品不存在")),
    }
}
fn _delete(conn: &mut PooledConn, product: &BaseInfos) -> Result<(), Response> {
    for table in CUSTOM_FIELD_INFOS[1] {
        conn.query_drop(format!("DELETE FROM {table} WHERE id = '{}'", product.id))?;
    }
    conn.query_drop(format!(
        "DELETE FROM product WHERE id = '{}' LIMIT 1",
        product.id
    ))?;
    if let Some(cover) = &product.cover {
        std::fs::remove_file(format!("resources/product/{cover}"))?;
    }
    Ok(())
}
