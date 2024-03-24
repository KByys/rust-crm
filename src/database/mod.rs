// #[forbid(unused)]
// mod table;
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
#[macro_export]
macro_rules! catch {
    ($result:expr => dup) => {
        match $result {
            Ok(ok) => Ok(ok),
            Err(err) => Err(match err {
                mysql::Error::MySqlError(e) if e.code == 1062 => {
                    $crate::Response::already_exist("重复添加")
                }
                e => $crate::Response::internal_server_error(e),
            }),
        }
    };
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
    conn.query_drop("BEGIN")?;
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

#[macro_export]
macro_rules! mysql_stmt {
    ($table:expr, $($idt:ident, )+) => {
        {
            let values = vec![$(stringify!($idt).to_string(), )+];
            // let params = mysql::params!{ $( stringify!($idt) => &$param.$idt, )+};
            let values1: String = values.iter().fold(String::new(),|out, v| {
                if out.is_empty() {
                    v.clone()
                } else {
                    format!("{},{}",out, v)
                }
            });

            let values2: String = values.iter().fold(String::new(),|out, v| {
                if out.is_empty() {
                    format!(":{}", v)
                } else {
                    format!("{},:{}",out, v)
                }
            });
            let stmt = format!("insert into {} ({values1}) values ({values2})", $table);
            println!("{stmt}");
            stmt
        }
    };



}

#[macro_export]
macro_rules! commit_or_rollback {
    (async $fn:expr, $conn:expr, $params:expr) => {{
        mysql::prelude::Queryable::query_drop($conn, "begin")?;
        match $fn($conn, $params).await {
            Ok(ok) => {
                mysql::prelude::Queryable::query_drop($conn, "commit")?;
                Ok(ok)
            }
            Err(e) => {
                mysql::prelude::Queryable::query_drop($conn, "rollback")?;
                Err(e)
            }
        }
    }};
    ($fn:expr, $conn:expr, $params:expr) => {{
        mysql::prelude::Queryable::query_drop($conn, "begin")?;
        match $fn($conn, $params) {
            Ok(ok) => {
                $conn.query_drop("COMMIT")?;
                Ok(ok)
            }
            Err(e) => {
                mysql::prelude::Queryable::query_drop($conn, "rollback")?;
                Err(e)
            }
        }
    }};
}

pub fn _c_or_r_more<F, T, P>(f: F, conn: &mut PooledConn, param: T, more: P) -> Result<(), Response>
where
    F: Fn(&mut PooledConn, T, P) -> Result<(), Response>,
{
    match f(conn, param, more) {
        Ok(_) => {
            conn.query_drop("COMMIT")?;
            Ok(())
        }
        Err(e) => {
            conn.query_drop("ROLLBACK")?;
            Err(e)
        }
    }
}

/// 连接数据库
pub fn get_conn() -> Result<PooledConn> {
    unsafe { Pool::new(MYSQL_URI.as_str())?.get_conn() }
}
// use table::Table;

use crate::{Response, MYSQL_URI};

pub fn create_table() -> Result<()> {
    let mut conn = get_conn()?;
    let sql = include_str!("./table.sql");
    conn.query_drop(sql)
}
