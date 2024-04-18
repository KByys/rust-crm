pub mod data;
mod router;
use data::Order;
mod customer;
mod invoice;
mod payment;
mod product;
mod ship;

use axum::{http::HeaderMap, routing::post, Json, Router};
use mysql::{params, prelude::Queryable, PooledConn};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback,
    database::get_conn,
    get_cache,
    libs::{cache::ORDER_CACHE, dser::deser_empty_to_none, gen_id, TimeFormat, TIME},
    log, mysql_stmt,
    pages::{
        account::{get_user, User},
        func::order::payment::Instalment,
    },
    parse_jwt_macro,
    perm::action::OtherGroup,
    verify_perms, Response, ResponseResult,
};

use self::{
    customer::Customer, invoice::Invoice, payment::Repayment, product::Product, ship::Ship,
};

pub fn order_router() -> Router {
    Router::new()
        .route("/order/add", post(add_order))
        .route("/order/query", post(query_order))
        .route("/order/update/status", post(update_order_status))
        .route("/order/update/order", post(update_order))
        .route("/order/finish/repayment", post(finish_repayment))
}

async fn add_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let mut order: Order = serde_json::from_value(value)?;
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{} 发起添加订单请求", user.department, user.name);
    commit_or_rollback!(async __add_order, &mut conn, (&mut order, &user))?;

    log!(
        "{}-{} 添加订单成功, 订单编号为：{}",
        user.department,
        user.name,
        order.number
    );
    ORDER_CACHE.clear();
    Ok(Response::empty())
}
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

macro_rules! verify_order {
    ($conn:expr, $param:expr, $user:expr) => {{
        if $param.repayment.is_invalid() {
            log!("系统拒绝{}添加订单的请求,instalment 非法", $user);
            return Err(Response::invalid_value(
                "instalment不能为空， 全款时也要有且只能有一期付款, 后一期回款日期必须大于之前的",
            ));
        } else if !$param.repayment.date_is_valid() {
            log!("系统拒绝{}添加订单的请求, 回款日期不能相同", $user);
            return Err(Response::invalid_value("回款日期不能相同"));
        }
        let price = $param.product.price($conn)?;
        let sum = $param.product.price_sum_with_discount(price);
        if sum != $param.repayment.sum() {
            log!("系统拒绝{}添加订单的请求, 付款金额不对", $user);
            return Err(Response::invalid_value("付款金额不对"));
        }
    }};
}

async fn __add_order(
    conn: &mut PooledConn,
    (order, user): (&mut Order, &User),
) -> Result<(), Response> {
    verify_order!(conn, order, user);
    let time = TIME::now()?;
    order.create_time = time.format(TimeFormat::YYYYMMDD_HHMMSS);
    if order.number.is_empty() {
        let name = format!(
            "{}{}{}",
            order.salesman.name, order.product.name, order.customer.name
        );
        order.number = gen_number!(conn, 0, &name);
    }
    order.id = gen_id(&time, &format!("order{}", user.name));

    match order.status {
        1 | 2 => {
            if order.transaction_date.is_none() {
                return Err(Response::invalid_value("transaction_date必须设置"));
            }
            if order.ship.shipped && order.ship.date.is_none() {
                return Err(Response::invalid_value("shipped为true时，date必须设置"));
            }
            if order.invoice.required {
                let stmt = mysql_stmt!("invoice", order_id, number, title, deadline, description,);
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
                        "title" => &order.invoice.title,
                        "deadline" => &order.invoice.deadline,
                        "description" => &order.invoice.description
                    },
                )?;
            }
        }
        _ => {
            // TODO
            // 目前没有什么要写
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
        repayment_model,
        payment_method,
        product,
        amount,
        discount,
        transaction_date,
        customer,
        address,
        purchase_unit,
        invoice_required,
        shipped,
        shipped_date,
        shipped_storehouse,
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
            "repayment_model" => &order.repayment.model,
            "customer" => &order.customer.id,
            "transaction_date" => &order.transaction_date,
            "address" => &order.customer.address,
            "purchase_unit" => &order.customer.purchase_unit,
            "invoice_required" => &order.invoice.required,
            "amount" => &order.product.amount,
            "shipped" => &order.ship.shipped,
            "shipped_date" => &order.ship.date,
            "shipped_storehouse" => &order.ship.storehouse
        },
    )?;

    order.repayment.smart_insert(&order.id, conn)?;
    Ok(())
}

#[derive(Deserialize)]
struct UpdateStatusParams {
    id: String,
    status: i32,
    #[serde(deserialize_with = "deser_empty_to_none")]
    transaction_date: Option<String>,
    invoice: Invoice,
    ship: Ship,
}
async fn update_order_status(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let mut param: UpdateStatusParams = serde_json::from_value(value)?;
    log!("{user} 请求修改订单{}状态", param.id);
    commit_or_rollback!(__update_order_status, &mut conn, &mut param, &user)?;
    log!("{user} 成功修改订单{}状态", param.id);
    ORDER_CACHE.clear();
    Ok(Response::ok(json!("修改订单状态成功")))
}
fn __update_order_status(
    conn: &mut PooledConn,
    param: &mut UpdateStatusParams,
    user: &User,
) -> Result<(), Response> {
    let order = query_order_by_id(conn, &param.id)?;
    if order.status == 2 {
        log!(
            "系统拒绝{}修改订单{}的状态，因为该订单处于已完成状态",
            user,
            param.id
        );
        return Err(Response::dissatisfy("该订单处于已完成状态, 不允许被修改"));
    } else if param.status < order.status {
        log!(
            "系统拒绝{}修改订单{}的状态，status不能减小，成交代收订单不能调整为意向订单",
            user,
            param.id
        );
        return Err(Response::dissatisfy("成交代收订单不能调整为意向订单"));
    } else if param.status > 0 && param.transaction_date.is_none() {
        log!(
            "系统拒绝{}修改订单{}的状态， status不为0时，transaction_date必须设置",
            user,
            param.id
        );
        return Err(Response::dissatisfy("transaction_date必须设置"));
    } else if param.status == 2 && order.status != 2 {
        let key: Option<i32> = conn.exec_first(
            "select 1 from order_instalment where order_id = ? and finish = 0 limit 1",
            (&param.id,),
        )?;
        if key.is_some() {
            log!(
                "系统拒绝{}修改订单{}的状态，修改status为2时，所有回款都必须已完成",
                user,
                param.id
            );
            return Err(Response::dissatisfy("无法完成订单，存在未完成的回款"));
        }
    }

    if !order.ship.shipped {
        if order.ship.shipped && (order.ship.storehouse.is_none() || order.ship.date.is_none()) {
            log!(
                "系统拒绝{}修改订单{}的状态，当设置成发货状态时，date和storehouse必须设置",
                user,
                param.id
            );
            return Err(Response::dissatisfy("ship的date和storehouse必须设置"));
        }
        conn.exec_drop("update order_data set  shipped= ?, shipped_date=?, shipped_storehouse=? where id = ? limit 1", 
    (param.ship.shipped, &param.ship.date, &param.ship.storehouse, &param.id))?;
    }

    conn.exec_drop(
        "update order_data set status = ?, transaction_date=? where id = ? limit 1",
        (param.status, &param.transaction_date, &param.id),
    )?;

    conn.exec_drop(
        "update order_data set invoice_required = ? where id = ? limit 1",
        (param.invoice.required, &param.id),
    )?;
    if order.invoice.required {
        if param.invoice.required {
            param.invoice.update(&param.id, conn)?;
        } else {
            param.invoice.delete(&param.id, conn)?;
        }
    } else if param.invoice.required {
        param.invoice.number = gen_number!(
            conn,
            1,
            format!("INV{}{}", order.salesman.name(), order.customer.name)
        );
        param.invoice.insert(&param.id, conn)?;
    }
    Ok(())
}

#[derive(Deserialize)]
struct UpdateOrderParam {
    id: String,
    ty: String,
    receipt_account: String,
    payment_method: String,
    repayment: Repayment,
    product: Product,
    customer: Customer,
}

async fn update_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let data: UpdateOrderParam = serde_json::from_value(value)?;
    log!("{user} 请求更新订单 {}", data.id);
    commit_or_rollback!(async __update_order, &mut conn, &data, &user)?;
    log!("{user} 成功更新订单 {}", data.id);
    ORDER_CACHE.clear();
    Ok(Response::ok(json!("更新订单成功")))
}
async fn __update_order(
    conn: &mut PooledConn,
    param: &UpdateOrderParam,
    user: &User,
) -> Result<(), Response> {
    verify_order!(conn, param, user);
    let order = query_order_by_id(conn, &param.id)?;
    if order.status == 2 {
        log!(
            "系统拒绝{}修改订单{}的请求，因为该订单处于已完成状态",
            user,
            param.id
        );
        return Err(Response::dissatisfy("该订单处于已完成状态, 不允许被修改"));
    }

    let instalment = Instalment::query(conn, &param.id)?;
    if instalment.len() != param.repayment.instalment.len() {
        log!(
            "系统拒绝{}修改订单{}的请求，分期数目不允许修改",
            user,
            param.id
        );
        return Err(Response::dissatisfy("分期数目不允许修改"));
    }
    for (i, item) in instalment.iter().enumerate() {
        let rep = &param.repayment.instalment[i];
        if !item.finish {
            conn.exec_drop(
                "update order_instalment 
            set interest=:inter, original_amount=:oa, date=:date 
            where order_id =:id and date ==:date and finish = 0 limit 1",
                params! {
                    "inter" => &rep.interest,
                    "oa" => &rep.original_amount,
                    "date" => &rep.date,
                    "id" => &param.id
                },
            )?;
        }
    }
    conn.exec_drop(
        "update order_data set ty=:ty, receipt_account=:ra, payment_method=:pm where id=:id limit 1
     ",
        params! {
            "ty" => &param.ty,
            "ra" => &param.receipt_account,
            "pm" => &param.payment_method,
            "id" => &param.id
        },
    )?;
    if order.status == 0 {
        conn.exec_drop(
            "update order_data set customer=?, address=?, purchase_unit=? where id = ? limit 1",
            (
                &param.customer.id,
                &param.customer.address,
                &param.customer.purchase_unit,
                &param.id,
            ),
        )?;

        conn.exec_drop(
            "update order_data set product=?, discount=?, amount=? where id = ? limit 1",
            (
                &param.product.id,
                &param.product.discount,
                &param.product.amount,
                &param.id,
            ),
        )?;
    }
    Ok(())
}

fn query_order_by_id(conn: &mut PooledConn, id: &str) -> Result<Order, Response> {
    if let Some(order) = get_cache!(ORDER_CACHE, "id", id) {
        if let Ok(order) = serde_json::from_value(order) {
            return Ok(order);
        }
    }
    let order: Option<Order> = conn.exec_first(
        "select o.*, u.name as salesman_name, c.name as customer_name, 
        c.company, p.name as product_name
        from order_data o
        join user u on u.id = o.salesman
        join customer c on c.id = o.customer
        join product p on p.id = o.product
        where o.id = ? limit 1
    ",
        (id,),
    )?;

    if let Some(order) = order {
        ORDER_CACHE
            .entry("id".into())
            .or_default()
            .insert(id.into(), json!(order));
        Ok(order)
    } else {
        log!("订单 {} 不存在", id);
        Err(Response::not_exist("订单不存在"))
    }
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    ty: u8,
    data: String,
}

async fn query_person_order(
    conn: &mut PooledConn,
    param: &QueryParams,
    user: &User,
) -> Result<Vec<Order>, Response> {
    let id = if param.data.eq("my") || user.id == param.data {
        log!("{}-{} 正在查询自己的订单", user.department, user.name);
        &user.id
    } else {
        let u = get_user(&param.data, conn).await?;
        log!(
            "{}-{} 正在查询 {}-{} 的订单",
            user.department,
            user.name,
            u.department,
            u.department
        );
        if u.department == user.department {
            if !verify_perms!(&user.role, OtherGroup::NAME, OtherGroup::QUERY_ORDER) {
                log!(
                    "{}-{} 查询 {}-{} 的订单失败，因为没有查看本部门其他成员订单的权限",
                    user.department,
                    user.name,
                    u.department,
                    u.department
                );
                return Err(Response::permission_denied());
            }
            &param.data
        } else if verify_perms!(
            &user.role,
            OtherGroup::NAME,
            OtherGroup::QUERY_ORDER,
            Some(["all"].as_slice())
        ) {
            log!(
                "{}-{} 查询 {}-{} 的订单失败，因为没有查看其他部门成员订单的权限",
                user.department,
                user.name,
                u.department,
                u.department
            );
            &param.data
        } else {
            log!(
                "{}-{} 查询 {}-{} 的订单失败，因为没有查看其他成员订单的权限",
                user.department,
                user.name,
                u.department,
                u.department
            );
            return Err(Response::permission_denied());
        }
    };

    conn.exec(
        "select o.*, u.name as salesman_name, c.name as customer_name, 
        c.company, p.name as product_name, p.price as product_price
        from order_data o
        join user u on u.id = o.salesman
        join customer c on c.id = o.customer
        join product p on p.id = o.product
        where o.salesman = ?
        order by o.create_time desc
    ",
        (&id,),
    )
    .map_err(Into::into)
}

async fn query_department_order(
    conn: &mut PooledConn,
    param: &QueryParams,
    user: &User,
) -> Result<Vec<Order>, Response> {
    if !verify_perms!(&user.role, OtherGroup::NAME, OtherGroup::QUERY_ORDER) {
        return Err(Response::permission_denied());
    }
    let depart = if param.data.eq("my") || user.department == param.data {
        &user.department
    } else if verify_perms!(
        &user.role,
        OtherGroup::NAME,
        OtherGroup::QUERY_ORDER,
        Some(["all"].as_slice())
    ) {
        &param.data
    } else {
        log!(
            "{}-{} 查询 {} 部门的订单失败，因为没有查看其他部门订单的权限",
            user.department,
            user.name,
            param.data
        );
        return Err(Response::permission_denied());
    };
    log!(
        "{}-{} 正在查询 {depart} 部门的订单",
        user.department,
        user.name
    );
    conn.exec(
        "select o.*, u.name as salesman_name, c.name as customer_name, 
        c.company, p.name as product_name, p.price as product_price
        from order_data o
        join user u on u.id = o.salesman and u.department = ?
        join customer c on c.id = o.customer
        join product p on p.id = o.product
        order by o.create_time desc
    ",
        (&depart,),
    )
    .map_err(Into::into)
}

async fn query_company_order(conn: &mut PooledConn, user: &User) -> Result<Vec<Order>, Response> {
    log!("{}-{} 正在查询全公司的订单", user.department, user.name);
    if !verify_perms!(
        &user.role,
        OtherGroup::NAME,
        OtherGroup::QUERY_ORDER,
        Some(["all"].as_slice())
    ) {
        log!(
            "{}-{} 查询全公司的订单失败，没有该权限",
            user.department,
            user.name
        );
        return Err(Response::permission_denied());
    }
    conn.query(
        "select o.*,
            u.name as salesman_name,
            c.name as customer_name,
            c.company,
            p.name as product_name, p.price as product_price
        from
            order_data o
            join user u on u.id = o.salesman
            join customer c on c.id = o.customer
            join product p on p.id = o.product
        order by
            o.create_time desc;",
    )
    .map_err(Into::into)
}

async fn query_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{} 请求查询订单", user.department, user.name);
    let param_str = value.to_string();
    let param: QueryParams = serde_json::from_value(value)?;

    let value = if let Some(value) = get_cache!(ORDER_CACHE, &uid, &param_str) {
        log!("缓存命中");
        value
    } else {
        log!("缓存未命中");
        let mut data = match param.ty {
            0 => query_person_order(&mut conn, &param, &user).await?,
            1 => query_department_order(&mut conn, &param, &user).await?,
            2 => query_company_order(&mut conn, &user).await?,
            _ => return Ok(Response::empty()),
        };
        for o in &mut data {
            o.repayment.smart_query(&o.id, &mut conn)?;
            if o.invoice.required {
                if let Some(invoice) = conn.query_first(format!(
                    "select *, 1 as required from invoice where order_id = '{}' limit 1",
                    o.id
                ))? {
                    o.invoice = invoice;
                }
            }
        }
        let value = json!(data);
        ORDER_CACHE
            .entry(uid)
            .or_default()
            .insert(param_str, value.clone());
        value
    };

    log!(
        "{user} 查询订单成功，共查询到{}条记录",
        value.as_array().map_or(0, |a| a.len())
    );
    Ok(Response::ok(value))
}

#[derive(Deserialize)]
struct PayParam {
    id: String,
    date: String,
}

async fn finish_repayment(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = get_conn()?;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let param: PayParam = serde_json::from_value(value)?;
    log!(
        "{} 请求完成订单{} -  回款日期为{} 的收款",
        user,
        param.id,
        param.date
    );
    let key: Option<i32> = conn.exec_first(
        "select 1 from order_data where id = ? and date < ? and finish = 0",
        (&param.id, &param.date),
    )?;
    if key.is_some() {
        log!(
            "系统拒绝 {} 对订单{} -  回款日期为{} 的收款，在此之前存在未完成的回款",
            user,
            param.id,
            param.date
        );
        return Err(Response::dissatisfy("必须要先完成之前的回款"));
    }
    conn.exec_drop(
        "update order_data set finish = 1 where id= ? and date = ? limit 1",
        (&param.id, &param.date),
    )?;

    ORDER_CACHE.clear();
    log!(
        "{} 成功完成订单{} -  回款日期为{} 的收款",
        user,
        param.id,
        param.date
    );
    Ok(Response::ok(json!("收款成功")))
}
