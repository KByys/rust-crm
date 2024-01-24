use axum::{extract::Path, http::HeaderMap, Json};
use mysql::{prelude::Queryable, PooledConn};
use serde_json::{json, Value};

use crate::{
    bearer,
    database::{c_or_r, get_conn, Database},
    debug_info,
    libs::time::TIME,
    parse_jwt_macro,
    response::Response,
    ResponseResult,
};

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
pub enum DataOptions {
    CustomerType,
    CustomerStatus,
    CustomerTag,
    Department,
    CustomerRole,
    Industry,
    CustomerSource,
    VisitTheme,
    /// 订单类型
    OrderType,
    /// 销售单位
    SalesUnit,
    /// 仓库
    StoreHouse,
    ProductType,
    ProductUnit,
    /// 收款方式
    Payment,
    /// 订单进度
    OrderProgress,
}
impl From<usize> for DataOptions {
    fn from(value: usize) -> Self {
        match value {
            0 => DataOptions::CustomerType,
            1 => DataOptions::CustomerStatus,
            2 => DataOptions::CustomerTag,
            3 => DataOptions::Department,
            4 => DataOptions::CustomerRole,
            5 => DataOptions::Industry,
            6 => DataOptions::CustomerSource,
            7 => DataOptions::VisitTheme,
            8 => DataOptions::OrderType,
            9 => DataOptions::SalesUnit,
            10 => DataOptions::StoreHouse,
            11 => DataOptions::ProductType,
            12 => DataOptions::ProductUnit,
            13 => DataOptions::Payment,
            14 => DataOptions::OrderProgress,
            _ => panic!("Invalid value {}", value),
        }
    }
}
impl Iterator for DataOptions {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        let n = *self as usize;
        
        if n < Self::max() - 1 {
            *self = DataOptions::from(n + 1);
            Some(n.into())
        } else {
            None
        }
    }
}
impl DataOptions {
    pub fn max() -> usize {
        15
    }
    pub fn first() -> Self {
        Self::CustomerType
    }
    pub fn table_name(&self) -> &'static str {
        match self {
            DataOptions::CustomerType => "customer_type",
            DataOptions::CustomerStatus => "customer_status",
            DataOptions::CustomerTag => "customer_tag",
            DataOptions::Department => "department",
            DataOptions::CustomerRole => "customer_role",
            DataOptions::Industry => "industry",
            DataOptions::CustomerSource => "customer_source",
            DataOptions::VisitTheme => "visit_theme",
            DataOptions::OrderType => "order_type",
            DataOptions::SalesUnit => "sales_unit",
            DataOptions::StoreHouse => "storehouse",
            DataOptions::ProductType => "product_type",
            DataOptions::ProductUnit => "product_unit",
            DataOptions::Payment => "payment",
            DataOptions::OrderProgress => "order_progress",
        }
    }
    pub fn table_statement(&self) -> String {
        format!(
            "CREATE TABLE IF NOT EXISTS {} (
            value VARCHAR(30) NOT NULL, 
            create_time VARCHAR(25),
            PRIMARY KEY (value)
        )",
            self.table_name()
        )
    }
}

#[derive(serde::Deserialize, Debug)]
struct ReceiveOptionInfo {
    ty: usize,
    info: OptionValue,
}

#[derive(serde::Deserialize, Default, Debug)]
#[serde(default)]
struct OptionValue {
    value: String,
    new_value: String,
    old_value: String,
    delete_value: String,
    next_value: String,
}
use crate::perm::{verify_permissions, action::OtherGroup};
macro_rules! parse_option {
    ($headers:expr, $value:expr, $begin:expr) => {
        {
            let bearer = bearer!(&$headers);
            let mut conn = get_conn()?;
            let id = parse_jwt_macro!(&bearer, &mut conn => true);
            
            let role: String = op::some!(conn.query_first(format!("SELECT role FROM user WHERE id = '{id}'"))?; ret Err(Response::not_exist("用户不存在")));
            if !role.eq("root") && !verify_permissions(&role, "other", OtherGroup::DROP_DOWN_BOX, None).await {
                return Err(Response::permission_denied())
            }

            if $begin {
                conn.query_drop("BEGIN")?;
            }
            let info: ReceiveOptionInfo = serde_json::from_value($value)?;
            (id, conn, info)
        }
    };
}

pub async fn insert_options(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let (id, mut conn, info) = parse_option!(headers, value, false);
    debug_info(format!("添加下拉框操作, 操作者：{}, 数据:{:?}", id, info));
    let time = TIME::now()?;
    if info.info.value.is_empty() {
        return Err(Response::invalid_value("value不能为空字符串"));
    }
    let opt = DataOptions::from(info.ty);
    conn.query_drop(format!(
        "INSERT IGNORE INTO {} (value, create_time) VALUES ('{}', '{}')",
        opt.table_name(),
        info.info.value,
        time.format(crate::libs::time::TimeFormat::YYYYMMDD_HHMMSS)
    ))?;

    Ok(Response::empty())
}
pub async fn update_option_value(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let (id, mut conn, info) = parse_option!(headers, value, true);
    debug_info(format!("修改下拉框操作, 操作者：{}, 数据:{:?}", id, info));
    conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    c_or_r(_update, &mut conn, &info, true)?;
    Ok(Response::empty())
}
fn _update(conn: &mut PooledConn, param: &ReceiveOptionInfo) -> Result<(), Response> {
    let opt = DataOptions::from(param.ty);

    if let DataOptions::Department = opt {
        conn.query_drop(format!(
            "UPDATE user SET department = '{}' WHERE department = '{}'",
            param.info.new_value, param.info.old_value
        ))?;
    }
    conn.query_drop(format!(
        "UPDATE {} SET value = '{}' WHERE value = '{}'",
        opt.table_name(),
        param.info.new_value,
        param.info.old_value
    ))?;
    Ok(())
}

pub async fn delete_option_value(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let (id, mut conn, info) = parse_option!(headers, value, true);
    debug_info(format!("修改下拉框操作, 操作者：{}, 数据:{:?}", id, info));
    if info.ty == DataOptions::Department as usize {
        let depart: Option<String> = conn.query_first(format!(
            "SELECT value FROM department WHERE value = '{}'",
            info.info.next_value
        ))?;
        if depart.is_none() {
            return Err(Response::invalid_value("next_value必须存在"));
        }
    }
    conn.query_drop(Database::SET_FOREIGN_KEY_0)?;
    c_or_r(_delete, &mut conn, &info, false)?;
    Ok(Response::empty())
}
fn _delete(conn: &mut PooledConn, param: &ReceiveOptionInfo) -> Result<(), Response> {
    let opt = DataOptions::from(param.ty);
    if let DataOptions::Department = opt {
        conn.query_drop(format!(
            "UPDATE user SET department = '{}' WHERE department = '{}'",
            param.info.next_value, param.info.delete_value
        ))?;
    }
    conn.query_drop(format!(
        "DELETE FROM {} WHERE value = '{}'",
        opt.table_name(),
        param.info.delete_value
    ))?;
    Ok(())
}

pub async fn query_option_value() -> ResponseResult {
    let mut data = Vec::new();
    let mut conn = get_conn()?;
    for opt in DataOptions::first() {
        let info: Vec<String> = conn.query_map(
            format!(
                "SELECT value FROM {} ORDER BY create_time",
                opt.table_name()
            ),
            |value| value,
        )?;
        let ty = opt as usize;
        data.push(json!({
            "ty": ty,
            "info": info
        }))
    }
    Ok(Response::ok(json!(data)))
}

pub async fn query_specific_info(Path(ty): Path<usize>) -> ResponseResult {
    if ty < DataOptions::max() {
        let mut conn = get_conn()?;
        let info: Vec<String> = conn.query_map(
            format!(
                "SELECT value FROM {} ORDER BY create_time",
                DataOptions::from(ty).table_name()
            ),
            |value| value,
        )?;
        Ok(Response::ok(json!({"ty": ty, "info": info})))
    } else {
        Err(Response::invalid_value(format!("ty: {} 错误", ty)))
    }
}
