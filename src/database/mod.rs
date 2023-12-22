#[forbid(unused)]
mod table;
use std::fmt::Display;

use mysql::{prelude::Queryable, Pool, PooledConn, Result};

pub struct Database;
impl Database {
    pub const SET_FOREIGN_KEY_0: &str = "SET foreign_key_checks = 0";
    pub const SET_FOREIGN_KEY_1: &str = "SET foreign_key_checks = 1";
    /// 主键已存在
    pub const DUPLICATE_KEY_ERROR_CODE: u16 = 1062;
    /// 外键无法匹配
    pub const FOREIGN_KEY_ERROR_CODE: u16 = 1452;
}

pub fn catch_some_mysql_error(code: u16, msg: impl Display, err: mysql::Error) -> Response {
    match err {
        mysql::Error::MySqlError(e) if e.code == code => {
            if code == Database::DUPLICATE_KEY_ERROR_CODE {
                Response::already_exist(msg)
            } else {
                Response::not_exist(msg)
            }
        }
        e => Response::internal_server_error(e),
    }
}
/// 成功提交，失败回滚
pub fn c_or_r<F, T>(
    f: F,
    conn: &mut PooledConn,
    param: T,
    start_check: bool,
) -> Result<(), Response>
where
    F: Fn(&mut PooledConn, T) -> Result<(), Response>,
{
    let result = match f(conn, param) {
        Ok(_) => {
            conn.query_drop("COMMIT")?;
            Ok(())
        }
        Err(e) => {
            conn.query_drop("ROLLBACK")?;
            Err(e)
        }
    };
    if start_check {
        conn.query_drop(Database::SET_FOREIGN_KEY_1)?;
    }
    result
}

/// 连接数据库
pub fn get_conn() -> Result<PooledConn> {
    unsafe { Pool::new(MYSQL_URI.as_str())?.get_conn() }
}
use table::Table;

use crate::{pages::DataOptions, Response, MYSQL_URI};
pub fn create_table() -> Result<()> {
    let mut conn = get_conn()?;
    // 创建下拉框选项的表格
    for value in DataOptions::first() {
        conn.query_drop(value.table_statement())?;
    }
    conn.query_drop("INSERT IGNORE INTO department VALUES ('总经办', '0000-00-00 00:00:00')")?;
    conn.query_drop(Table::USER_TABLE)?;
    // 设置token黑名单明个
    conn.query_drop(Table::TOKEN)?;
    conn.query_drop(
        "INSERT IGNORE INTO payment (value, create_time) VALUES 
            ('现金', '0000-00-00 00:00:00'), 
            ('银行转账', '0000-00-00 00:00:01'), 
            ('对公转账', '0000-00-00 00:00:02')",
    )?;
    conn.query_drop("INSERT IGNORE INTO storehouse (value, create_time) VALUES ('主仓库', '0000-00-00 00:00:00')")?;
    conn.query_drop(Table::CUSTOMER_TABLE)?;
    conn.query_drop(Table::CUSTOMER_LOGIN_TABLE)?;
    conn.query_drop(Table::APPOINTMENT_TABLE)?;
    conn.query_drop(Table::SING_TABLE)?;

    // 自定义字段，客户和产品
    for fields in crate::pages::CUSTOM_FIELDS {
        for table in fields {
            conn.query_drop(format!(
                "CREATE TABLE IF NOT EXISTS {table}(
                value VARCHAR(30) NOT NULL,
                create_time VARCHAR(25),
                PRIMARY KEY (value)
            )"
            ))?;
        }
    }
    // 自定义下拉字段的选项
    for (i, field) in crate::pages::CUSTOM_BOX_FIELDS.iter().enumerate() {
        conn.query_drop(format!(
            "CREATE TABLE IF NOT EXISTS {field}(
                display VARCHAR(30) NOT NULL,
                value VARCHAR(30) NOT NULL,
                create_time VARCHAR(25),
                PRIMARY KEY (display, value),
                FOREIGN KEY (display) REFERENCES {}(value)
            )",
            crate::pages::CUSTOM_FIELDS[i][2]
        ))?;
    }
    // 存放客户和产品的自定义字段的值
    for fields in crate::pages::CUSTOM_FIELD_INFOS {
        for table in fields {
            // TODO  暂时不考虑添加外键
            // 文本 和 时间
            conn.query_drop(format!(
                "CREATE TABLE IF NOT EXISTS {table} (
                    id VARCHAR(15) NOT NULL,
                    display VARCHAR(30) NOT NULL,
                    value VARCHAR(30)
                )"
            ))?;
        }
    }
    conn.query_drop(Table::PRODUCT_TABLE)?;
    Ok(())
}
