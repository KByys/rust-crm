use axum::{extract::Path, http::HeaderMap, Json};
use mysql::{prelude::Queryable, PooledConn};
use serde_json::{json, Value};

use crate::{
    bearer,
    database::{c_or_r, catch_some_mysql_error, get_conn, Database},
    debug_info,
    libs::{
        perm::Identity,
        time::{TimeFormat, TIME},
    },
    parse_jwt_macro, Response, ResponseResult,
};

#[derive(serde::Deserialize, Debug)]
pub struct CustomInfos {
    ty: usize,
    display: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    old_value: String,
    #[serde(default)]
    new_value: String,
}
/// 自定义字段
pub const CUSTOM_FIELDS: [[&str; 3]; 2] = [
    [
        "customize_customer_text",
        "customize_customer_time",
        "customize_customer_box",
    ],
    [
        "customize_product_text",
        "customize_product_time",
        "customize_product_box",
    ],
];
/// 自定义字段的下拉框选项
pub const CUSTOM_BOX_FIELDS: [&str; 2] = [
    "customize_customer_box_option",
    "customize_product_box_option",
];
/// 客户和产品的自定义字段的值
pub const CUSTOM_FIELD_INFOS: [[&str; 3]; 2] = [
    [
        "customize_customer_text_infos",
        "customize_customer_time_infos",
        "customize_customer_box_infos",
    ],
    [
        "customize_product_text_infos",
        "customize_product_time_infos",
        "customize_product_box_infos",
    ],
];

fn verify_perm(headers: HeaderMap, conn: &mut PooledConn) -> Result<String, Response> {
    let bearer = bearer!(&headers);
    let id = parse_jwt_macro!(&bearer, conn => true);
    if let Identity::Boss = Identity::new(&id, conn)? {
        Ok(id)
    } else {
        Err(Response::permission_denied())
    }
}
#[derive(Clone, Copy)]
#[repr(usize)]
enum CustomizeFieldType {
    Text,
    Time,
    Box,
}
impl CustomizeFieldType {
    pub fn new(s: &str) -> Result<Self, Response> {
        match s {
            "0" => Ok(Self::Text),
            "1" => Ok(Self::Time),
            "2" => Ok(Self::Box),
            _ => Err(Response::invalid_value(format!(
                "display的值 `{}`，非法",
                s
            ))),
        }
    }
}
pub async fn insert_custom_field(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let id = verify_perm(headers, &mut conn)?;
    debug_info(format!("添加自定义字段，操作者：{}，数据：{:?}", id, value));
    let data: CustomInfos = serde_json::from_value(value)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    } else if data.value.is_empty() {
        // 字段为空字符串则忽略
        return Err(Response::empty());
    }
    CustomizeFieldType::new(&data.display)?;
    conn.query_drop("BEGIN")?;
    c_or_r(_insert_field, &mut conn, &data, false)?;
    Ok(Response::empty())
}
fn _insert_field(conn: &mut PooledConn, param: &CustomInfos) -> Result<(), Response> {
    let field = CustomizeFieldType::new(&param.display).unwrap();
    let create_time = TIME::now().unwrap().format(TimeFormat::YYYYMMDD_HHMMSS);
    let table = CUSTOM_FIELDS[param.ty][field as usize];
    let is_exist = conn
        .query_first::<String, String>(format!(
            "SELECT value FROM {} WHERE value = '{}'",
            table, param.value
        ))?
        .is_some();
    if is_exist {
        return Ok(());
    }
    conn.query_drop(format!(
        "INSERT INTO {} ( value, create_time) VALUES ('{}', '{}')",
        table, param.value, create_time
    ))?;
    let id: Vec<String> = if param.ty == 0 {
        // if !customers_id.is_empty() {
        //     let table = CUSTOM_FIELD_INFOS[param.ty][field as usize];
        //     let mut values: String = customers_id
        //         .iter()
        //         .map(|id| format!("('{}' ,'{}', ''),", param.value, id))
        //         .collect();
        //     values.pop();
        //     let query = format!("INSERT INTO {table} (display, id, value) VALUES {}", values);
        //     println!("{}", query);
        //     conn.query_drop(format!(
        //         "INSERT INTO {table} (display, id, value) VALUES {}",
        //         values
        //     ))?;
        // }
        conn.query_map("SELECT id FROM customer", |s| s)?
    } else {
        conn.query_map("SELECT id FROM product", |s| s)?
    };

    if !id.is_empty() {
        let table = CUSTOM_FIELD_INFOS[param.ty][field as usize];
        let mut values: String = id
            .iter()
            .map(|id| format!("('{}' ,'{}', ''),", param.value, id))
            .collect();
        values.pop();
        let query = format!("INSERT INTO {table} (display, id, value) VALUES {}", values);
        println!("{}", query);
        conn.query_drop(format!(
            "INSERT INTO {table} (display, id, value) VALUES {}",
            values
        ))?;
    }
    Ok(())
}

pub async fn insert_box_option(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let id = verify_perm(headers, &mut conn)?;
    debug_info(format!(
        "添加自定义下拉字段的选项，操作者：{}，数据：{:?}",
        id, value
    ));
    let data: CustomInfos = serde_json::from_value(value)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    } else if data.value.is_empty() {
        // 字段为空字符串则忽略
        return Err(Response::empty());
    }
    let create_time = TIME::now()?.format(TimeFormat::YYYYMMDD_HHMMSS);

    let table = CUSTOM_BOX_FIELDS[data.ty];
    conn.query_drop(format!(
        "INSERT INTO {table} (display, value, create_time) VALUES ('{}', '{}', '{create_time}')",
        data.display, data.value
    ))
    .map_err(|err| {
        catch_some_mysql_error(
            Database::FOREIGN_KEY_ERROR_CODE,
            format!("没有 ‘{}’这个自定义下拉字段", data.display),
            err,
        )
    })?;
    Ok(Response::empty())
}

pub async fn update_custom_field(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let id = verify_perm(headers, &mut conn)?;
    debug_info(format!(
        "修改自定义下拉字段，操作者：{}，数据：{:?}",
        id, value
    ));
    let data: CustomInfos = serde_json::from_value(value)?;
    println!("{:#?}", data);
    CustomizeFieldType::new(&data.display)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    } else if data.new_value.is_empty() || data.old_value.is_empty() {
        return Err(Response::invalid_value("new_value 或 old_value 不能为空"));
    }
    conn.query_drop("BEGIN")?;
    conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    c_or_r(_update_custom_field, &mut conn, &data, true)?;
    Ok(Response::empty())
}

fn _update_custom_field(conn: &mut PooledConn, param: &CustomInfos) -> Result<(), Response> {
    let field = CustomizeFieldType::new(&param.display).unwrap();
    // 更新字段
    let table = CUSTOM_FIELDS[param.ty][field as usize];
    conn.query_drop(format!(
        "UPDATE {table} SET value = '{}' WHERE value = '{}'",
        param.new_value, param.old_value
    ))?;
    println!(
        "UPDATE {table} SET value = '{}' WHERE value = '{}'",
        param.new_value, param.old_value
    );
    // 更新客户或产品对应的字段值
    let table = CUSTOM_FIELD_INFOS[param.ty][field as usize];
    conn.query_drop(format!(
        "UPDATE {table} SET display = '{}' WHERE display = '{}'",
        param.new_value, param.old_value
    ))?;
    if let CustomizeFieldType::Box = field {
        // 更新下拉字段选项对应的字段
        let table = CUSTOM_BOX_FIELDS[param.ty];
        conn.query_drop(format!(
            "UPDATE {table} SET display = '{}' WHERE display = '{}'",
            param.new_value, param.old_value
        ))?;
    }
    Ok(())
}

pub async fn update_box_option(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let id = verify_perm(headers, &mut conn)?;
    debug_info(format!(
        "修改自定义下拉字段的选项，操作者：{}，数据：{:?}",
        id, value
    ));
    let data: CustomInfos = serde_json::from_value(value)?;
    if data.new_value.is_empty() {
        return Ok(Response::empty());
    }
    let table = CUSTOM_BOX_FIELDS[data.ty];
    conn.query_drop(format!(
        "UPDATE {} SET value = '{}' WHERE value = '{}' AND display = '{}'",
        table, data.new_value, data.old_value, data.display
    ))?;
    Ok(Response::empty())
}

pub async fn delete_custom_field(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let id = verify_perm(headers, &mut conn)?;
    debug_info(format!(
        "删除自定义下拉字段，操作者：{}，数据：{:?}",
        id, value
    ));
    let data: CustomInfos = serde_json::from_value(value)?;
    CustomizeFieldType::new(&data.display)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    }
    conn.query_drop("BEGIN")?;
    conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    c_or_r(_delete_custom_field, &mut conn, &data, true)?;
    Ok(Response::empty())
}

fn _delete_custom_field(conn: &mut PooledConn, param: &CustomInfos) -> Result<(), Response> {
    let field = CustomizeFieldType::new(&param.display).unwrap();
    // 删除字段
    let table = CUSTOM_FIELDS[param.ty][field as usize];
    conn.query_drop(format!(
        "DELETE FROM {table} WHERE value = '{}'",
        param.value
    ))?;
    // 删除客户或产品对应的字段值
    let table = CUSTOM_FIELD_INFOS[param.ty][field as usize];
    conn.query_drop(format!(
        "DELETE FROM {table} WHERE display = '{}'",
        param.value
    ))?;
    if let CustomizeFieldType::Box = field {
        // 删除下拉字段选项对应的字段
        let table = CUSTOM_BOX_FIELDS[param.ty];
        conn.query_drop(format!(
            "DELETE FROM {table} WHERE display = '{}'",
            param.value
        ))?;
    }
    Ok(())
}

pub async fn delete_box_option(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    let id = verify_perm(headers, &mut conn)?;
    debug_info(format!(
        "删除自定义下拉字段的选项，操作者：{}，数据：{:?}",
        id, value
    ));
    let data: CustomInfos = serde_json::from_value(value)?;
    let table = CUSTOM_BOX_FIELDS[data.ty];
    conn.query_drop(format!(
        "DELETE FROM {} WHERE value = '{}' AND display = '{}'",
        table, data.value, data.display
    ))?;
    Ok(Response::empty())
}

pub async fn get_custom_info_with(Path(ty): Path<usize>) -> ResponseResult {
    let mut conn = get_conn()?;
    let query_values = |table, conn: &mut PooledConn| {
        conn.query_map(
            format!("SELECT value FROM {table} ORDER BY create_time"),
            |value: String| value,
        )
    };
    let text_infos = query_values(CUSTOM_FIELDS[ty][0], &mut conn)?;
    let time_infos = query_values(CUSTOM_FIELDS[ty][1], &mut conn)?;
    let _box_infos: Vec<_> = query_values(CUSTOM_FIELDS[ty][2], &mut conn)?;
    let mut box_infos = Vec::new();
    let table = CUSTOM_BOX_FIELDS[ty];
    for text in _box_infos {
        let t = conn.query_map(
            format!(
                "SELECT value FROM {table} WHERE display = '{}' ORDER BY create_time",
                text
            ),
            |text: String| text,
        )?;
        box_infos.push(json!({
            "display": text,
            "values": t
        }));
    }
    let value = json!({
        "ty": ty,
        "text_infos": text_infos,
        "time_infos": time_infos,
        "box_infos": box_infos
    });
    Ok(Response::ok(value))
}

pub async fn get_custom_info() -> ResponseResult {
    let mut conn = get_conn()?;
    let mut data = Vec::new();
    for ty in 0..=1 {
        let query_values = |table, conn: &mut PooledConn| {
            conn.query_map(
                format!("SELECT value FROM {table} ORDER BY create_time"),
                |value: String| value,
            )
        };
        let text_infos = query_values(CUSTOM_FIELDS[ty][0], &mut conn)?;
        let time_infos = query_values(CUSTOM_FIELDS[ty][1], &mut conn)?;
        let _box_infos: Vec<_> = query_values(CUSTOM_FIELDS[ty][2], &mut conn)?;
        let mut box_infos = Vec::new();
        let table = CUSTOM_BOX_FIELDS[ty];
        for text in _box_infos {
            let t = conn.query_map(
                format!(
                    "SELECT value FROM {table} WHERE display = '{}' ORDER BY create_time",
                    text
                ),
                |text: String| text,
            )?;
            box_infos.push(json!({
                "display": text,
                "values": t
            }));
        }
        data.push(json!({
            "ty": ty,
            "text_infos": text_infos,
            "time_infos": time_infos,
            "box_infos": box_infos
        }));
    }
    Ok(Response::ok(json!(data)))
}
