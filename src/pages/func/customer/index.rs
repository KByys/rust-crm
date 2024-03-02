use std::collections::HashMap;

use axum::{extract::Path, http::HeaderMap, routing::post, Json, Router};
use chrono::{Days, TimeZone};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};

use crate::{
    bearer, catch,
    database::{c_or_r, get_conn},
    libs::{gen_id, TimeFormat, TIME},
    pages::{
        account::get_user,
        func::{__update_custom_fields, get_custom_fields},
    },
    parse_jwt_macro,
    perm::{action::CustomerGroup, verify_permissions},
    Field, Response, ResponseResult,
};

pub fn customer_router() -> Router {
    Router::new()
        .route("/customer/list/data", post(query_customer))
        .route("/customer/full/data/:id", post(query_full_data))
        .route("/customer/update", post(update_customer))
        .route("/customer/add", post(insert_customer))
}

use crate::libs::dser::{
    deser_empty_to_none, deserialize_bool_to_i32, deserialize_mm_dd, serialize_i32_to_bool,
    serialize_null_to_default,
};

// pub static mut STATIC_CUSTOMER_LEVEL: CustomerLevel = CustomerLevel::new();
// pub struct CustomerLevel {
//     inner: Option<HashMap<String, String>>,
// }
// impl CustomerLevel {
//     pub const fn new() -> Self {
//         CustomerLevel { inner: None }
//     }
//     pub fn init(&mut self) {
//         let buf = std::fs::read_to_string("data/level").expect("读取配置文件data/level失败");
//         let map = serde_json::from_str(&buf).expect("配置文件data/level已遭到损坏");
//         self.inner = Some(map);
//     }
// }

#[derive(Serialize, Deserialize, Debug, FromRow)]
pub struct Customer {
    pub id: String,
    pub smartphone: String,
    pub name: String,
    pub company: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    is_share: i32,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
    level: String,
    chat: String,
    need: String,
    create_time: String,
    fax: String,
    post: String,
    industry: String,
    #[serde(deserialize_with = "deserialize_mm_dd")]
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    role: String,
    ty: String,
    tag: String,
    pub salesman: Option<String>,
    pub visited_count: usize,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub next_visit_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_visited_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_transaction_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub push_to_sea_date: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub pop_from_sea_date: Option<String>,
    pub custom_fields: CustomCustomerData,
}


#[derive(Serialize, Debug, FromRow)]
pub struct ListData {
    id: String,
    smartphone: String,
    name: String,
    company: String,
    salesman: Option<String>,
    level: String,
    #[serde(serialize_with = "serialize_null_to_default")]
    next_visit_time: Option<String>,
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
    address: String,
    ty: String,
    status: String,
    create_time: String,
    visited_count: usize,
    #[serde(serialize_with = "serialize_null_to_default")]
    last_visited_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    last_transaction_time: Option<String>,
}
#[derive(Default, Debug)]
pub struct CustomCustomerData {
    pub inner: HashMap<String, Vec<Field>>,
}

impl<'de> Deserialize<'de> for CustomCustomerData {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(Self {
            inner: Deserialize::deserialize(deserializer)?,
        })
    }
}

impl Serialize for CustomCustomerData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}
impl From<String> for CustomCustomerData {
    fn from(_: String) -> Self {
        Self::default()
    }
}
impl mysql::prelude::FromValue for CustomCustomerData {
    type Intermediate = String;
}

fn __insert_customer(conn: &mut PooledConn, table: &InsertParams) -> Result<(), Response> {
    let time = TIME::now()?;
    let id = gen_id(&time, &table.name);
    let create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    catch!(conn.exec_drop(
                "INSERT INTO customer 
                (id, create_time, smartphone, name, company, is_share, sex, chat, need,
                fax, post, industry, birthday, address, remark, status, source,
                role, ty, tag, level) 
                VALUES
                (:id, :create_time, :smartphone, :name, :company, :is_share, :sex, :chat, :need,
                :fax, :post, :industry, :birthday, :address, :remark, :status, :source,
                :role, :ty,  :tag, :level)",
                params! {
                    "id" => &id,
                    "create_time" => create_time,
                    "smartphone" => table.smartphone.trim(),
                    "name" => table.name.trim(),
                    "company" => table.company.trim(),
                    "is_share" => table.is_share,
                    "sex" => table.sex,
                    "chat" => &table.chat,
                    "need" => &table.need,
                    "fax" => &table.fax,
                    "post" => &table.post,
                    "industry" => &table.industry,
                    "birthday" => &table.birthday,
                    "address" => &table.address,
                    "remark" => &table.remark,
                    "status" => &table.status,
                    "source" => &table.source,
                    "role" => &table.role,
                    "ty" => &table.ty,
                    "tag" => &table.tag,
                    "level" => &table.level
                }
            ) => dup)?;
    conn.exec_drop("INSERT INTO extra_customer_data (id, salesman, last_transaction_time,
        push_to_sea_date, pop_from_sea_date, added_date) VALUES (:id, :salesman, :last_transaction_time,
        :push_to_sea_date, :pop_from_sea_date, :added_date) ", params! {
        "id" => &id,
        "salesman" => &table.salesman,
        "last_transaction_time" => mysql::Value::NULL,
        "push_to_sea_date" => mysql::Value::NULL,
        "pop_from_sea_date" => mysql::Value::NULL,
        "added_date" => time.format(TimeFormat::YYYYMMDD)
    })?;

    crate::pages::func::__insert_custom_fields(conn, &table.custom_fields, 0, &id)?;
    Ok(())
}

#[derive(Deserialize, Debug)]
struct InsertParams {
    smartphone: String,
    name: String,
    company: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    is_share: i32,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    sex: i32,
    chat: String,
    need: String,
    fax: String,
    post: String,
    industry: String,
    #[serde(deserialize_with = "deserialize_mm_dd")]
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    level: String,
    role: String,
    ty: String,
    tag: String,
    #[serde(deserialize_with = "deser_empty_to_none")]
    salesman: Option<String>,
    custom_fields: HashMap<String, Vec<Field>>,
}
// fn check_user_exists(conn: &mut PooledConn, id: &str) -> Result<bool, Response> {
//     Ok(conn
//         .query_first::<String, String>(format!(
//             "SELECT 1 FROM user u WHERE u.id = '{id}' AND
//             NOT EXISTS (SELECT 1 FROM leaver l WHERE l.id=u.id) LIMIT 1"
//         ))?
//         .is_some())
// }

async fn insert_customer(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: InsertParams = serde_json::from_value(value)?;
    let user = get_user(&id, &mut conn)?;
    println!(
        "添加客户，{}-{} : {:#?}",
        user.name, user.smartphone, params
    );

    if !verify_permissions(
        &user.role,
        "customer",
        CustomerGroup::ENTER_CUSTOMER_DATA,
        None,
    )
    .await
    {
        return Err(Response::permission_denied());
    }

    c_or_r(__insert_customer, &mut conn, &params, false)?;
    Ok(Response::empty())
}
#[derive(Debug, FromRow, Deserialize, Serialize)]
struct Colleague {
    id: String,
    name: String,
    phone: String,
}
#[derive(Debug)]
struct CustomerColleagues {
    inner: Vec<Colleague>,
}
impl Serialize for CustomerColleagues {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}
impl From<String> for CustomerColleagues {
    fn from(_value: String) -> Self {
        Self { inner: Vec::new() }
    }
}
impl mysql::prelude::FromValue for CustomerColleagues {
    type Intermediate = String;
}
#[derive(Serialize, Debug, FromRow)]
struct FullCustomerData {
    id: String,
    smartphone: String,
    name: String,
    company: String,
    #[serde(serialize_with = "serialize_i32_to_bool")]
    is_share: i32,
    #[serde(serialize_with = "serialize_i32_to_bool")]
    sex: i32,
    level: String,
    chat: String,
    need: String,
    create_time: String,
    fax: String,
    post: String,
    industry: String,
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    role: String,
    ty: String,
    tag: String,
    pub salesman: Option<String>,
    pub visited_count: usize,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub next_visit_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_visited_time: Option<String>,
    #[serde(serialize_with = "serialize_null_to_default")]
    pub last_transaction_time: Option<String>,
    pub custom_fields: CustomCustomerData,
}
fn __query_full_data(
    conn: &mut PooledConn,
    id: &str,
) -> Result<Option<FullCustomerData>, Response> {
    let time = TIME::now()?;
    let today = time.format(TimeFormat::YYYYMMDD);
    // 会出现重复列，目前测试数据正确
    let query = format!(
        "SELECT DISTINCT c.*, ex.salesman, ex.last_transaction_time, 
            MIN(app.appointment) as next_visit_time, COUNT(cou.id) as visited_count,
            MAX(cou.appointment) as last_visited_time, 1 as custom_fields,
            1 as colleagues
             FROM customer c 
            JOIN extra_customer_data ex ON ex.id = c.id 
            LEFT JOIN appointment app ON app.customer = c.id AND app.salesman=ex.salesman
                AND app.appointment > '{today}' AND app.finish_time IS NULL
            LEFT JOIN appointment cou ON cou.customer = c.id AND cou.salesman=ex.salesman
                AND cou.finish_time IS NOT NULL
            WHERE c.id = '{id}'
            GROUP BY c.id, app.id, cou.id"
    );
    println!("{}", query);

    let mut data: Option<FullCustomerData> = conn.query_first(query)?;
    if let Some(d) = &mut data {
        d.custom_fields = get_custom_fields(conn, &d.id, 0)?;
    }
    Ok(data)
}

async fn query_full_data(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    parse_jwt_macro!(&bearer, &mut conn => true);
    let data = __query_full_data(&mut conn, &id)?;
    Ok(Response::ok(json!(data)))
}

#[derive(Deserialize)]
struct QueryParams {
    status: Option<String>,
    ty: Option<String>,
    ap: i32,
    appointment: u64,
    // visited_days: i32,
    added_days: i32,
    #[allow(unused)]
    is_share: Value,
    salesman: String,
    department: String,
}

macro_rules! __convert {
    ($arg:expr) => {
        match &$arg {
            Some(s) => op::ternary!(s.is_empty() => "IS NOT NULL".into(); format!("='{s}'")),
            None => "=''".into()
        }
    };
    ($time:expr, $days:expr, $local:expr, $name:expr) => {
        if $days < 0 {
            format!("IS NULL OR {} IS NOT NULL", $name)
        } else {
            let t = op::some!($local.checked_sub_days(Days::new($days as u64));
                ret Err(Response::invalid_value("天数错误")));
            format!(">= '{}'",TIME::from(t).format(TimeFormat::YYYYMMDD))
        }
    };
    ($param:expr, $time:expr, $local:expr => appointment) => {
        match $param.ap {
            0 => (0 , "".into()),
            1 => {
                let t = op::some!($local.checked_sub_days(Days::new($param.appointment));
                    ret Err(Response::invalid_value("天数错误")));
                (1, format!("a.finish_time >= '{}'", TIME::from(t).format(TimeFormat::YYYYMMDD)))
            }
            2 => {
                let t = op::some!($local.checked_add_days(Days::new($param.appointment));
                    ret Err(Response::invalid_value("天数错误")));
               (1, format!("a.finish_time IS NULL AND (a.appointment >= '{}' AND a.appointment <= '{} 24:00:00')",
                    $time.format(TimeFormat::YYYYMMDD), TIME::from(t).format(TimeFormat::YYYYMMDD)))
            }
            _ => return Err(Response::invalid_value("ap错误"))
        }
    };
    // ()
    ($sales:expr, $depart:expr, $u:expr, $conn:expr; auto) => {
        if $sales.is_empty() {
            if !$depart.is_empty() {
                if ($depart.eq(&$u.department) && verify_permissions(&$u.role, "customer", "query", None).await) ||
                    verify_permissions(&$u.role, "customer", "query", Some(&["all"])).await {
                        ("IS NOT NULL".to_owned(), format!("='{}'", $depart))
                } else{
                    return Err(Response::permission_denied())
                }
            } else {
                if verify_permissions(&$u.role, "customer", "query", Some(&["all"])).await {
                    ("IS NOT NULL".to_owned(), "IS NOT NULL".to_owned())
                } else{
                    return Err(Response::permission_denied())
                }
            }
        } else if $sales.eq("my") {
            (format!("='{}'", $u.id), "IS NOT NULL".to_owned())
        } else {
            let sl = get_user($sales, $conn)?;
            if (sl.department.eq(&$u.department) && verify_permissions(&$u.role, "customer", "query", None).await) ||
                verify_permissions(&$u.role, "customer", "query", Some(&["all"])).await {
                    (format!("='{}'", $sales), "IS NOT NULL".to_owned())
            } else{
                return Err(Response::permission_denied())
            }
        }
    }

}
async fn __query_customer_list_data(
    conn: &mut PooledConn,
    params: &QueryParams,
    id: &str,
) -> Result<Vec<ListData>, Response> {
    let status = __convert!(params.status);
    let ty = __convert!(params.ty);
    let time = TIME::now()?;
    let local = chrono::Local.timestamp_nanos(time.naos() as i64);
    let (ot, appoint) = __convert!(&params, &time, local => appointment);
    let added_time = __convert!(time, params.added_days, local, "ex.added_date");
    let ap = if ot == 0 {
        String::new()
    } else {
        format!("JOIN appointment a ON a.customer=c.id AND a.salesman=ex.salesman AND ({appoint})")
    };
    let u = get_user(id, conn)?;

    let (salesman, department) =
        __convert!(params.salesman.as_str(), params.department, u, conn; auto);
    let today = time.format(TimeFormat::YYYYMMDD);
    let query = format!(
        "SELECT c.id, c.smartphone, c.name, c.company, 
        c.sex, c.ty, c.status, c.create_time, c.level, c.address, ex.salesman, COUNT(cou.id) as visited_count,
        MAX(cou.finish_time) AS last_visited_time,
        ex.last_transaction_time, MIN(app.appointment) as next_visit_time
        FROM customer c JOIN extra_customer_data ex ON ex.id=c.id AND (ex.salesman {salesman}) 
        AND (ex.added_date {added_time})
        JOIN user u ON u.id=ex.salesman AND (u.department {department}) 
        {ap}
        LEFT JOIN appointment app ON app.customer=c.id AND app.salesman=ex.salesman AND app.appointment>'{today}' AND app.finish_time IS NULL
        LEFT JOIN appointment cou ON cou.customer=c.id AND cou.salesman=ex.salesman AND cou.finish_time IS NOT NULL
        WHERE (c.status {status}) AND (c.ty {ty})
        GROUP BY c.id
        "
    );
    println!("{}", query);
    let list = conn.query_map(query, |data| data)?;
    Ok(list)
}

async fn query_customer(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);

    let params: QueryParams = serde_json::from_value(value)?;
    let list = __query_customer_list_data(&mut conn, &params, &id).await?;
    Ok(Response::ok(json!(list)))
}

#[derive(Deserialize, Debug)]
struct UpdateParams {
    id: String,
    smartphone: String,
    name: String,
    company: String,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    is_share: i32,
    #[serde(deserialize_with = "deserialize_bool_to_i32")]
    sex: i32,
    level: String,
    chat: String,
    need: String,
    fax: String,
    post: String,
    industry: String,
    #[serde(deserialize_with = "deserialize_mm_dd")]
    birthday: String,
    address: String,
    remark: String,
    status: String,
    source: String,
    role: String,
    ty: String,
    tag: String,
    custom_fields: HashMap<String, Vec<Field>>,
}
pub fn check_user_customer(
    id: &str,
    customer: &str,
    conn: &mut PooledConn,
) -> Result<(), Response> {
    let flag: Option<String> = conn.query_first(format!(
        "SELECT 1 FROM customer c 
            JOIN extra_customer_data d ON d.id=c.id AND d.salesman='{id}'
            WHERE c.id='{customer}'"
    ))?;
    if flag.is_some() {
        Ok(())
    } else {
        Err(Response::permission_denied())
    }
}
async fn update_customer(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let id = parse_jwt_macro!(&bearer, &mut conn => true);
    let params: UpdateParams = serde_json::from_value(value)?;
    let user = get_user(&id, &mut conn)?;
    println!(
        "更新客户，{}-{} : {:#?}",
        user.name, user.smartphone, params
    );
    check_user_customer(&id, &params.id, &mut conn)?;
    if !verify_permissions(&user.role, "customer", "update_customer_data", None).await {
        return Err(Response::permission_denied());
    }

    c_or_r(__update_customer, &mut conn, &params, false)?;
    Ok(Response::empty())
}
fn __update_customer(conn: &mut PooledConn, params: &UpdateParams) -> Result<(), Response> {
    conn.exec_drop(
        format!(
            "UPDATE customer SET smartphone=:smartphone, name=:name, company=:company,
        is_share=:is_share, sex=:sex, chat=:chat, level=:level,
        need=:need, fax=:fax, post=:post, industry=:industry, birthday=:birthday,
        address=:address, remark=:remark, status=:status, 
        source=:source, role=:role, ty=:ty, tag=:tag WHERE id = '{}' LIMIT 1",
            params.id
        ),
        params! {
                    "smartphone" => &params.smartphone,
                    "name" => &params.name,
                    "company" => &params.company,
                    "is_share" => params.is_share,
                    "sex" => params.sex,
                    "chat" => &params.chat,
                    "need" => &params.need,
                    "level" => &params.level,
                    "fax" => &params.fax,
                    "post" => &params.post,
                    "industry" => &params.industry,
                    "birthday" => &params.birthday,
                    "address" => &params.address,
                    "remark" => &params.remark,
                    "status" => &params.status,
                    "source" => &params.source,
                    "role" => &params.role,
                    "ty" => &params.ty,
                    "tag" => &params.tag
        },
    )?;
    __update_custom_fields(conn, &params.custom_fields, 0, &params.id)?;
    Ok(())
}
