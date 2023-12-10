use axum::{http::HeaderMap, Json};
use mysql::{prelude::Queryable, PooledConn};
use mysql_common::frunk::labelled::chars::P;
use serde_json::Value;

use crate::{
    bearer,
    database::{catch_some_mysql_error, commit_or_rollback, get_conn, Database},
    debug_info,
    libs::{
        perm::Identity,
        time::{TimeFormat, TIME},
    },
    parse_jwt_macro, Response, ResponseResult,
};

#[derive(serde::Deserialize)]
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
    commit_or_rollback(_insert_field, &mut conn, &data, false)?;
    Ok(Response::empty())
}
fn _insert_field(conn: &mut PooledConn, param: &CustomInfos) -> mysql::Result<()> {
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
        "INSERT INTO {} (value, create_time) VALUES ('{}', '{}')",
        table, param.value, create_time
    ))?;
    if param.ty == 0 {
        let customers_id: Vec<String> = conn.query_map("SELECT id FROM customer", |s| s)?;
        let table = CUSTOM_FIELD_INFOS[param.ty][field as usize];
        let mut values: String = customers_id
            .iter()
            .map(|id| format!("('{}', ''),", id))
            .collect();
        values.pop();
        conn.query_drop(format!("INSERT INTO {table} (id, value) VALUES {}", values))?;
    } else {
        // TODO
        todo!()
    }
    Ok(())
}

pub async fn insert_custom_box_field(
    headers: HeaderMap,
    Json(value): Json<Value>,
) -> ResponseResult {
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
    CustomizeFieldType::new(&data.display)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    } else if data.new_value.is_empty() || data.old_value.is_empty() {
        return Err(Response::invalid_value("new_value 或 old_value 不能为空"));
    }
    conn.query_drop("BEGIN")?;
    conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    commit_or_rollback(_update_custom_field, &mut conn, &data, true)?;
    Ok(Response::empty())
}

fn _update_custom_field(conn: &mut PooledConn, param: &CustomInfos) -> mysql::Result<()> {
    let field = CustomizeFieldType::new(&param.display).unwrap();
    // 更新字段
    let table = CUSTOM_FIELDS[param.ty][field as usize];
    conn.query_drop(format!(
        "UPDATE {table} SET value = '{}' WHERE value = '{}'",
        param.new_value, param.old_value
    ))?;
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
        "修改自定义下拉字段，操作者：{}，数据：{:?}",
        id, value
    ));
    let data: CustomInfos = serde_json::from_value(value)?;
    CustomizeFieldType::new(&data.display)?;
    if data.ty > 1 {
        return Err(Response::invalid_value("ty 大于 1"));
    }
    conn.query_drop("BEGIN")?;
    conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    commit_or_rollback(_delete_custom_field, &mut conn, &data, true)?;
    Ok(Response::empty())
}

fn _delete_custom_field(conn: &mut PooledConn, param: &CustomInfos) -> mysql::Result<()> {
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
        "DELETE FROM {table} SET WHERE display = '{}'",
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
