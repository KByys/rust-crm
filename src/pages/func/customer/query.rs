use axum::{http::HeaderMap, Json};
use serde_json::Value;

use crate::{
    bearer, database::get_conn, libs::perm::Identity, parse_jwt_macro, Response, ResponseResult,
};

const QUERY_CUSTOMER_TABLE: &str = "";

use super::FixedCustomerInfos;
#[repr(i32)]
enum FilterTy {
    /// 未分类
    Uncategorized = -2,
    All = -1,
    /// 今日需回访
    TodayFollowupVisit = 0,
    /// 今日已回访
    VisitedToday,
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

pub async fn query_all_customer_infos(
    headers: HeaderMap,
    Json(value): Json<Value>,
) -> ResponseResult {
    let bearer = bearer!(&headers);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let data: ReceiveInfo = serde_json::from_value(value)?;
    let mut customers = match data.scope.ty {
        0 => query_my_customers(&data.filter, &id)?,
        1 => query_share_customers(&data.filter)?,
        2 => {
            let depart = match Identity::new(&id, &mut conn)? {
                Identity::Boss => data.scope.info.clone(),
                Identity::Administrator(_, depart) => depart,
                _ => return Err(Response::permission_denied()),
            };
            query_department_customers(&data.filter, &depart)?
        }
        ty => return Err(Response::invalid_value(format!("scope的ty: {} 非法", ty))),
    };
    for (info, _) in &mut customers {
        sort(info, data.sort)
    }
    todo!()
}

fn sort(data: &mut [FixedCustomerInfos], sort: usize) {
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
            _ => std::cmp::Ordering::Equal,
        }
    })
}

fn complete_customer_infos(data: Vec<FixedCustomerInfos>) -> mysql::Result<Vec<Value>> {
    todo!()
}

fn query_share_customers(filter: &Info) -> mysql::Result<Vec<(Vec<FixedCustomerInfos>, String)>> {
    todo!()
}
fn query_my_customers(
    filter: &Info,
    id: &str,
) -> mysql::Result<Vec<(Vec<FixedCustomerInfos>, String)>> {
    todo!()
}
fn query_department_customers(
    filter: &Info,
    department: &str,
) -> mysql::Result<Vec<(Vec<FixedCustomerInfos>, String)>> {
    todo!()
}
