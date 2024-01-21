use std::fmt::Display;

use axum::{http::HeaderMap, Json};
use chrono::prelude::TimeZone;
use chrono::{Days, Months};
use mysql::{prelude::Queryable, PooledConn};
use op::ternary;
use serde_json::{json, Value};

use crate::do_if;
use crate::libs::time::{TimeFormat, TIME};
use crate::pages::account::get_user;
use crate::perm::verify_permissions;
use crate::{
    bearer, database::get_conn, pages::CUSTOM_FIELD_INFOS, parse_jwt_macro, Response,
    ResponseResult, TextInfos,
};

use super::{CCInfos, Customer, FCInfos};
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
#[repr(i32)]
enum FilterType {
    /// 未分类
    UNCATEGORIZED = -2,
    ALL = -1,
    /// 今日需回访
    TODAY_FOLLOW_UP_VISIT = 0,
    /// 今日已回访
    VISITED_TODAY,
    /// 三天内已拜访
    VISITED_THREE_DAYS_AGO,
    VISITED_WEEK_AGO,
    VISITED_HALF_MONTH_AGO,
    VISITED_MONTH_AGO,
    /// 今日新增
    ADDED_TODAY,
    /// 一周内新增
    ADDED_WEEK_AGO,
    ADDED_HALF_MONTH_AGO,
    ADDED_MONTH_AGO,
}

impl From<&str> for FilterType {
    fn from(value: &str) -> Self {
        match value {
            "-2" => Self::UNCATEGORIZED,
            "-1" => Self::ALL,
            "0" => Self::TODAY_FOLLOW_UP_VISIT,
            "1" => Self::VISITED_TODAY,
            "2" => Self::VISITED_THREE_DAYS_AGO,
            "3" => Self::VISITED_WEEK_AGO,
            "4" => Self::VISITED_HALF_MONTH_AGO,
            "5" => Self::VISITED_MONTH_AGO,
            "6" => Self::ADDED_TODAY,
            "7" => Self::ADDED_WEEK_AGO,
            "8" => Self::ADDED_HALF_MONTH_AGO,
            "9" => Self::ADDED_MONTH_AGO,
            _ => Self::ALL,
        }
    }
}

#[derive(serde::Deserialize)]
struct Info {
    ty: usize,
    info: String,
}

#[derive(serde::Deserialize)]
struct ReceiveInfo {
    #[serde(default)]
    page_size: usize,
    #[serde(default)]
    current_page: usize,
    status: Option<String>,
    sort: usize,
    filter: Info,
    scope: Info,
}
/// 直接查询客户信息，对比分页查询
pub async fn qc_infos(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: ReceiveInfo = serde_json::from_value(value)?;
    let customers = query_all_customers_infos(id, &data, &mut conn).await?;
    Ok(Response::ok(json!(customers)))
}

/// 分页查询客户信息
///
pub async fn qc_infos_with_pages(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: ReceiveInfo = serde_json::from_value(value)?;
    let customers = query_all_customers_infos(id, &data, &mut conn).await?;
    if data.page_size == 0 {
        return Err(Response::invalid_value("page_size不允许为0"));
    };
    let total = customers.len();
    let total_pages = total / data.page_size + do_if!(total % data.page_size == 0 => 0, 1);
    let start = data.current_page * data.current_page;
    let end = start + data.page_size;
    if start >= total {
        let empty: Vec<()> = Vec::new();
        Ok(Response::ok(json!({
            "total": total,
            "total_pages": total_pages,
            "current_page": data.current_page,
            "page_size": data.page_size,
            "data": empty
        })))
    } else {
        let slice = do_if!(end > total => &customers[start..], &customers[start..end]);
        Ok(Response::ok(json!({
            "total": total,
            "total_pages": total_pages,
            "current_page": data.current_page,
            "page_size": data.page_size,
            "data": slice
        })))
    }
}
async fn query_all_customers_infos(
    id: String,
    data: &ReceiveInfo,
    conn: &mut PooledConn,
) -> Result<Vec<Customer>, Response> {
    let status = match &data.status {
        Some(status) => {
            do_if!(status.is_empty() => "status is not null".to_owned(), format!("status = '{}'", status))
        }
        _ => "status = ''".to_string(),
    };
    let mut customers = match data.scope.ty {
        0 => qm_customers(&data.filter, &id, &status, conn)?,
        1 => qs_customers(&data.filter, &status, conn)?,
        2 => {
            let u = get_user(&id, conn)?;
            let depart = {
                if data.scope.info.is_empty() {
                    let flag = verify_permissions(&u.role, "customer", "query", None).await;
                    ternary!(flag => &u.department; return Err(Response::permission_denied()))
                } else {
                    ternary!(u.department.eq(&data.scope.info) => &u.department; {
                        let flag = verify_permissions(&u.role, "customer", "query", Some(&["all"])).await;
                        ternary!(flag => &u.department;return Err(Response::permission_denied()))
                    })
                }
            };
            qc_with_d(&data.filter, depart, &status, conn)?
        }
        ty => return Err(Response::invalid_value(format!("scope的ty: {} 非法", ty))),
    };
    sort(&mut customers, data.sort);
    get_custom_infos(customers, conn).map_err(Into::into)
}

fn sort(data: &mut [FCInfos], sort: usize) {
    data.sort_by(|v1, v2| {
        use rust_pinyin::get_pinyin;
        match sort {
            // 姓名排序
            0 => get_pinyin(&v1.name).cmp(&get_pinyin(&v2.name)),
            // 公司排序
            1 => get_pinyin(&v1.company).cmp(&get_pinyin(&v2.company)),
            // 创建时间
            2 => v1.create_time.cmp(&v2.create_time),
            // 下次预约时间
            3 => v1.next_visit_time.cmp(&v2.next_visit_time),
            // 拜访次数
            4 => v2.visited_count.cmp(&v1.visited_count),
            // 最近拜访
            5 => v2.last_visited_time.cmp(&v1.last_visited_time),
            // 久未联系
            6 => v1.last_visited_time.cmp(&v2.last_visited_time),
            // 成交时间
            7 => v1.last_transaction_time.cmp(&v2.last_transaction_time),
            _ => std::cmp::Ordering::Equal,
        }
    })
}

fn get_custom_infos(data: Vec<FCInfos>, conn: &mut PooledConn) -> mysql::Result<Vec<Customer>> {
    let mut customers = Vec::new();
    for info in data {
        let mut custom_infos = CCInfos::default();
        for i in 0..=2 {
            let infos: Vec<TextInfos> = conn.query_map(
                format!(
                    "SELECT display, value FROM {} WHERE id = '{}'",
                    CUSTOM_FIELD_INFOS[0][i], info.id
                ),
                |info| info,
            )?;
            *custom_infos.get_mut(i) = infos;
        }
        customers.push(Customer {
            fixed_infos: info,
            custom_infos,
        });
    }
    Ok(customers)
}
type VecCustomer = mysql::Result<Vec<FCInfos>>;
/// 查询共享的客户信息
fn qs_customers(f: &Info, status: &str, conn: &mut PooledConn) -> VecCustomer {
    conn.query_map(
        query_statement(format!("is_share = 0 AND {}", gen_filter(f, status))),
        |f| f,
    )
}
/// 查询我的客户信息
fn qm_customers(f: &Info, id: &str, status: &str, conn: &mut PooledConn) -> VecCustomer {
    let query = query_statement(format!("salesman = '{}' AND {}", id, gen_filter(f, status)));
    println!("{}", query);
    conn.query_map(query, |f| f)
}
/// 查询指定部门的客户信息
fn qc_with_d(f: &Info, d: &str, status: &str, conn: &mut PooledConn) -> VecCustomer {
    let query = format!(
        "SELECT c.* FROM customer c JOIN user u ON u.id = c.salesman AND u.department = '{}' 
                WHERE {}",
        d,
        gen_filter(f, status)
    );
    conn.query_map(query, |f| f)
}
/// 生成一些类同的过滤条件
fn gen_filter(f: &Info, status: &str) -> String {
    let filter = if f.ty == 0 {
        let now = TIME::now().unwrap();
        let local = chrono::Local.timestamp_nanos(now.naos() as i64);
        match FilterType::from(f.info.as_str()) {
            FilterType::UNCATEGORIZED => format!("ty = '' AND {}", status),
            FilterType::ALL => status.to_owned(),
            FilterType::TODAY_FOLLOW_UP_VISIT => {
                format!(
                    "next_visit_time > '{}' AND {}",
                    now.format(TimeFormat::YYYYMMDD),
                    status
                )
            }
            FilterType::ADDED_TODAY => format!(
                "create_time > '{}' AND {status}",
                now.format(TimeFormat::YYYYMMDD)
            ),
            FilterType::ADDED_WEEK_AGO => {
                let week = local.checked_sub_days(Days::new(7)).unwrap();
                let time = TIME::from(week);
                format!(
                    "create_time >= '{}' AND {status}",
                    time.format(TimeFormat::YYYYMMDD)
                )
            }
            FilterType::ADDED_HALF_MONTH_AGO => {
                let half_of_month = local.checked_sub_days(Days::new(15)).unwrap();
                let time = TIME::from(half_of_month);
                format!(
                    "create_time >= '{}' AND {status}",
                    time.format(TimeFormat::YYYYMMDD)
                )
            }
            FilterType::ADDED_MONTH_AGO => {
                let month = local.checked_add_months(Months::new(1)).unwrap();
                let time = TIME::from(month);
                format!(
                    "create_time >= '{}' AND {status}",
                    time.format(TimeFormat::YYYYMMDD)
                )
            }
            FilterType::VISITED_TODAY => {
                format!(
                    "last_visited_time > '{}' AND {status}",
                    now.format(TimeFormat::YYYYMMDD)
                )
            }
            FilterType::VISITED_THREE_DAYS_AGO => {
                let three_days_ao = local.checked_sub_days(Days::new(3)).unwrap();
                let time = TIME::from(three_days_ao);
                format!(
                    "last_visited_time >= '{}' AND {status}",
                    time.format(TimeFormat::YYYYMMDD)
                )
            }
            FilterType::VISITED_WEEK_AGO => {
                let week = local.checked_sub_days(Days::new(7)).unwrap();
                let time = TIME::from(week);
                format!(
                    "last_visited_time >= '{}' AND {status}",
                    time.format(TimeFormat::YYYYMMDD)
                )
            }
            FilterType::VISITED_HALF_MONTH_AGO => {
                let half_of_month = local.checked_sub_days(Days::new(15)).unwrap();
                let time = TIME::from(half_of_month);
                format!(
                    "last_visited_time >= '{}' AND {status}",
                    time.format(TimeFormat::YYYYMMDD)
                )
            }
            FilterType::VISITED_MONTH_AGO => {
                let month = local.checked_add_months(Months::new(1)).unwrap();
                let time = TIME::from(month);
                format!(
                    "last_visited_time >= '{}' AND {status}",
                    time.format(TimeFormat::YYYYMMDD)
                )
            }
        }
    } else {
        format!("ty = '{}' AND {}", f.info, status)
    };
    format!("{} AND scope = 0", filter)
}
fn query_statement(f: impl Display) -> String {
    format!("SELECT DISTINCT * FROM customer WHERE {f}")
}
