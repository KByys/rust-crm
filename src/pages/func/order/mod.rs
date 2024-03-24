mod customer;
mod invoice;
mod payment;
mod product;
mod ship;
use std::collections::HashMap;

use axum::{http::HeaderMap, routing::post, Json, Router};
use mysql::{
    params,
    prelude::{FromRow, Queryable},
    FromRowError, PooledConn,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback,
    common::Person,
    database::get_conn,
    libs::{dser::deser_empty_to_none, TimeFormat, TIME},
    mysql_stmt,
    pages::account::{get_user, User},
    parse_jwt_macro, Response, ResponseResult,
};

use self::{
    customer::Customer, invoice::Invoice, payment::Repayment, product::Product, ship::Ship,
};

pub fn order_router() -> Router {
    Router::new().route("/order/add", post(add_order))
    .route("/order/query", post(query_order))
}

#[derive(Deserialize, Serialize)]
struct Order {
    #[serde(default)]
    id: String,
    #[serde(default)]
    create_time: String,
    number: String,
    status: i32,
    ty: String,
    #[serde(deserialize_with = "deser_empty_to_none")]
    transaction_date: Option<String>,
    receipt_account: String,
    salesman: Person,
    payment_method: String,
    repayment: Repayment,
    product: Product,
    customer: Customer,
    invoice: Invoice,
    ship: Vec<Ship>,
}

macro_rules! get {
    ($map:expr, $name:expr) => {
        mysql::prelude::FromValue::from_value($map.get($name)?.clone())
    };
}

impl FromRow for Order {
    fn from_row_opt(row: mysql::Row) -> Result<Self, mysql::FromRowError>
    where
        Self: Sized,
    {
        let columns = row.columns();
        let _row = row.clone();
        let values = row.unwrap();
        let map: HashMap<String, _> = values
            .into_iter()
            .enumerate()
            .map(|(i, item)| (columns[i].name_str().to_string(), item))
            .collect();
        let result: Option<Order> = op::catch!(Some(Self {
            id: get!(map, "id"),
            create_time: get!(map, "create_time"),
            number: get!(map, "number"),
            status: get!(map, "status"),
            ty: get!(map, "ty"),
            transaction_date: get!(map, "transaction_date"),
            receipt_account: get!(map, "receipt_account"),
            salesman: Person {
                name: get!(map, "salesman_name"),
                id: get!(map, "salesman")
            },
            payment_method: get!(map, "payment_method"),
            repayment: Repayment {
                model: get!(map, "model"),
                instalment: Vec::new()
            },
            product: Product {
                discount: get!(map, "discount"),
                id: get!(map, "product"),
                name: get!(map, "product_name"),
                inventory: Vec::new(),
            },
            customer: Customer {
                id: get!(map, "customer"),
                address: get!(map, "address"),
                name: get!(map, "customer_name"),
                company: get!(map, "company"),
                purchase_unit: get!(map, "purchase_unit")
            },
            invoice: Invoice {
                required: get!(map, "required"),
                ..Default::default()
            },
            ship: Vec::new(),
        }));
        if let Some(order) = result {
            Ok(order)
        } else {
            Err(FromRowError(_row))
        }
    }
}
async fn add_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let mut order: Order = serde_json::from_value(value)?;
    let user = get_user(&uid, &mut conn)?;

    commit_or_rollback!(async __add_order, &mut conn, (&mut order, &user))?;

    Ok(Response::empty())
}
// #[macro_export]
macro_rules! gen_number {
    ($conn:expr, $ty:expr, $name:expr) => {
        {
            use rust_pinyin::get_pinyin;
            let pinyin = get_pinyin(&format!("{}", $name));
            let number = $conn
                .exec_first(
                    "select num from order_num where name = ? and ty = ?",
                    (&pinyin,  $ty),
                )?
                .unwrap_or(0)
                + 1;
            $conn.exec_drop("insert into order_num (name, ty, num) values (:name, :ty, :num) on duplicate key update num = :new_num", params! {
                "name" => &pinyin,
                "ty" => $ty,
                "num" => number,
                "new_num" => number
            })?;
            format!("NO.{}{:0>7}", pinyin, number)
        }
    };
}

async fn __add_order(
    conn: &mut PooledConn,
    (order, _user): (&mut Order, &User),
) -> Result<(), Response> {
    let time = TIME::now()?;
    order.create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    if order.number.is_empty() {
        let name = format!(
            "{}{}{}",
            order.salesman.name, order.product.name, order.customer.name
        );
        order.number = gen_number!(conn, &name, 0);
    }
    check_repayment(conn, order)?;
    match order.status {
        1 | 2 => {
            if order.transaction_date.is_none() {
                return Err(Response::invalid_value("transaction_date必须设置"));
            }
            if order.ship.is_empty() || order.ship.len() != order.product.inventory.len() {
                return Err(Response::invalid_value("ship未设置或与产品未对应"));
            }
            let not_match = (0..order.ship.len())
                .any(|i| order.product.inventory[i].storehouse != order.ship[i].storehouse);
            if not_match {
                return Err(Response::invalid_value("ship与产品inventory没有对应"));
            }
            if order.invoice.required {
                let stmt = mysql_stmt!(
                    "invoice",
                    order_id,
                    number,
                    ty,
                    title,
                    deadline,
                    description,
                );
                let number = gen_number!(
                    conn,
                    1,
                    format!("INV{}{}", order.salesman.name(), order.customer.name)
                );
                conn.exec_drop(
                    stmt,
                    params! {
                        "order_id" => &order.id,
                        "number" => number,
                        "ty" =>  &order.invoice.ty,
                        "title" => &order.invoice.title,
                        "deadline" => &order.invoice.deadline,
                        "description" => &order.invoice.description
                    },
                )?;
            }
        }
        _ => {
            // TODO
        }
    }

    let stmt = mysql_stmt!(
        "order_data",
        id,
        number,
        create_time,
        status,
        ty,
        receipt_account,
        salesman,
        payment_method,
        product,
        discount,
        customer,
        address,
        purchase_unit,
        invoice_required,
    );
    conn.exec_drop(
        stmt,
        params! {
            "id" => &order.id,
            "number" =>  &order.number,
            "create_time" => &order.create_time,
            "status" => &order.status,
            "ty" => &order.ty,
            "receipt_account" => &order.receipt_account,
            "salesman" => &order.salesman.id,
            "payment_method" => &order.payment_method,
            "product" => &order.product.id,
            "discount" => &order.product.discount,
            "customer" => &order.customer.id,
            "address" => &order.customer.address,
            "purchase_unit" => &order.customer.purchase_unit,
            "invoice_required" => &order.invoice.required
        },
    )?;

    order.product.add_inventory(conn, &order.id)?;
    order.repayment.smart_insert(&order.id, conn)?;
    Ok(())
}
fn check_repayment(conn: &mut PooledConn, order: &Order) -> Result<(), Response> {
    if order.repayment.model != 0 {
        let price = order.product.price(conn)?;
        let sum = order.product.price_sum_with_discount(price);
        if sum != order.repayment.sum().unwrap_or_default() {
            return Err(Response::invalid_value("分期付款总金额不正确"));
        }
    }

    Ok(())
}
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct QueryParams {}

async fn query_order(header: HeaderMap, Json(_value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let mut data: Vec<Order> = conn.exec(
        "select o.*, u.name as salesman_name, c.name as customer_name, 
        c.company, p.name as product_name
        from order o
        join user u on u.id = o.salesman
        join customer c on c.id = o.customer
        join product p on p.id = o.product
        order by desc o.create_time
        where o.id = ?
    ",
        (&uid,),
    )?;
    for o in &mut data {
        o.repayment.smart_query(&o.id, &mut conn)?;
        o.product.query_inventory(&o.id, &mut conn)?;
        if o.invoice.required {
            if let Some(invoice) = conn.query_first(format!(
                "select *, 1 as required from invoice where order_id = '{}' limit 1",
                o.id
            ))? {
                o.invoice = invoice;
            }
        }
        o.ship = conn.query(format!(
            "select * from ship where order_id = '{}' order by storehouse",
            o.id
        ))?;
    }
    Ok(Response::ok(json!(data)))
}
