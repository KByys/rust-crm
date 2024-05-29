pub mod data;
mod router;
use data::Order;
mod customer;
mod invoice;
mod payment;
mod product;
mod ship;

use axum::{
    extract::{Multipart, Path},
    http::HeaderMap,
    routing::{delete, get, post},
    Json, Router,
};
use mysql::{params, prelude::Queryable, PooledConn};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{
    bearer, commit_or_rollback,
    database::{DB, DBC},
    get_cache,
    libs::{cache::ORDER_CACHE, gen_file_link, gen_id, parse_multipart, TimeFormat, TIME},
    log, mysql_stmt,
    pages::account::{get_user, User},
    parse_jwt_macro,
    perm::action::OtherGroup,
    response::BodyFile,
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
        .route("/order/upload/image/:id", post(upload_order_file))
        .route("/order/delete/:id", delete(delete_order))
        .route("/order/get/commission", get(get_commission))
        .route("/order/set/commission/:value", post(set_commission))
        .route("/order/get/img/:url", get(get_order_file))
}
async fn get_commission() -> ResponseResult {
    Ok(Response::ok(json!({
        "commission": crate::get_commission()?
    })))
}
async fn set_commission(header: HeaderMap, Path(value): Path<i32>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    if user.role.eq("root") {
        crate::set_commission(value)?;
        log!("已修改提成为{value}%");
        Ok(Response::ok(json!("成功修改提成")))
    } else {
        log!("仅老总权限可设置提成");
        Err(Response::permission_denied())
    }
}
async fn upload_order_file(
    header: HeaderMap,
    Path(id): Path<String>,
    part: Multipart,
) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let data = parse_multipart(part).await?;
    let Some(f) = data.files.first() else {
        log!("/order/upload/image/{id}，没有接收到附件信息");
        return Err(Response::invalid_value("没有接收到附件信息"));
    };
    let Some::<Option<String>>(file) = conn.query_first(format!(
        "select file from order_data where id = '{}' and salesman = '{uid}' limit 1",
        id
    ))?
    else {
        log!("上传附件失败，该订单不存在或权限不足");
        return Err(Response::permission_denied());
    };
    let time = TIME::now()?;
    let link = gen_file_link(&time, f.filename());
    std::fs::write(format!("resources/order/{link}"), &f.bytes)?;
    conn.query_drop(format!(
        "update order_data set file = '{link}' where id = '{id}' limit 1"
    ))?;
    if let Some(path) = file {
        std::fs::remove_file(path).unwrap_or_default();
    }
    log!("添加订单附件成功");
    Ok(Response::ok(json!("添加订单附件成功")))
}

async fn add_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let mut order: Order = serde_json::from_value(value)?;
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{} 发起添加订单请求", user.department, user.name);
    commit_or_rollback!(async __add_order, &mut conn, &mut order, &user)?;

    log!(
        "{}-{} 添加订单成功, 订单编号为：{}",
        user.department,
        user.name,
        order.number
    );
    ORDER_CACHE.clear();
    Ok(Response::ok(json!({"id": order.id})))
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
    }};
}

async fn __add_order<'err>(
    conn: &mut DB<'err>,
    order: &mut Order,
    user: &User,
) -> Result<(), Response> {
    verify_order!(conn, order, user);
    order.product.query_price(conn, 0, "")?;
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
        pre_price,
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
            "pre_price" => &order.product.price,
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
    for item in &mut order.repayment.instalment {
        item.finish = order.status == 2;
    }
    order.repayment.smart_insert(&order.id, conn)?;
    Ok(())
}

#[derive(Deserialize)]
struct UpdateStatusParams {
    id: String,
    status: i32,
    #[serde(default)]
    #[serde(deserialize_with = "crate::libs::dser::op_deser_yyyy_mm_dd_hh_mm_ss")]
    transaction_date: Option<String>,
    #[serde(default)]
    invoice: Invoice,
    #[serde(default)]
    repayment: Repayment,
    #[serde(default)]
    ship: Ship,
}

async fn update_order_status(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let mut param: UpdateStatusParams = serde_json::from_value(value)?;
    log!("{user} 请求修改订单{}状态", param.id);
    commit_or_rollback!(_new_update_order_status, &mut conn, &mut param, &user)?;
    log!("{user} 成功修改订单{}状态", param.id);
    ORDER_CACHE.clear();
    Ok(Response::ok(json!("修改订单状态成功")))
}

fn _new_update_order_status(
    conn: &mut PooledConn,
    param: &mut UpdateStatusParams,
    user: &User,
) -> Result<(), Response> {
    let mut order = query_order_by_id(conn, &param.id)?;
    if user.id != order.salesman.id {
        log!(
            "错误，{} 无法修改 {}-{} 的订单",
            user,
            order.salesman.id,
            order.salesman.name()
        );
        return Err(Response::permission_denied());
    }
    match order.status {
        0 if param.status > 0 => {
            if param.transaction_date.is_none() {
                log!(
                    "系统拒绝{}修改订单{}的状态， status不为0时，transaction_date必须设置",
                    user,
                    param.id
                );
                return Err(Response::dissatisfy("transaction_date必须设置"));
            } else if param.ship.shipped
                && (param.ship.date.is_none() || param.ship.storehouse.is_none())
            {
                log!(
                    "系统拒绝{}修改订单{}的状态，当设置成发货状态时，date和storehouse必须设置",
                    user,
                    param.id
                );
                return Err(Response::dissatisfy("ship的date和storehouse必须设置"));
            }
            order.product.query_price(conn, 0, &param.id)?;
            if param.repayment.is_invalid() || !param.repayment.date_is_valid() {
                return Err(Response::invalid_value("回款数据错误"));
            } else if order.product.price_sum_with_discount() != param.repayment.sum() {
                return Err(Response::invalid_value("回款金额错误"));
            } else if param.repayment.instalment.iter().any(|inv|inv.date.is_empty()) {
                return Err(Response::invalid_value("回款日期必须设置"));
            }
            for item in &mut param.repayment.instalment {
                item.finish = param.status == 2;
            }
            conn.exec_drop(
                "delete from order_instalment where order_id = ?",
                (&param.id,),
            )?;
            param.repayment.smart_insert(&param.id, conn)?;

            if param.invoice.required {
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
                        "title" => &param.invoice.title,
                        "deadline" => &param.invoice.deadline,
                        "description" => &param.invoice.description
                    },
                )?;
            }
            conn.exec_drop(
                "update order_data set transaction_date=:td, 
                        shipped=:sd, shipped_date=:sdd, 
                        status=:status,
                        repayment_model=:model,
                        shipped_storehouse=:ssh, invoice_required=:ir where id = :id limit 1",
                params! {
                    "sd" => param.ship.shipped,
                    "sdd" => &param.ship.date,
                    "td" => &param.transaction_date,
                    "ssh" => &param.ship.storehouse,
                    "ir" => &param.invoice.required,
                    "id" => &param.id,
                    "model" => &param.repayment.model,
                    "status" => &param.status
                },
            )?;
            Ok(())
        }
        1 if param.status == 2 => {
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
            conn.exec_drop(
                "update order_data set status = 2 where id = ? limit 1",
                (order.id.as_str(),),
            )?;
            Ok(())
        }
        2 => {
            log!(
                "系统拒绝{}修改订单{}的状态，因为该订单处于已完成状态",
                user,
                param.id
            );
            Err(Response::dissatisfy("该订单处于已完成状态, 不允许被修改"))
        }
        i if i == param.status => Ok(()),
        _ => {
            log!("系统拒绝{}修改订单{}的状态，status值非法", user, param.id);
            Err(Response::dissatisfy("status非法"))
        }
    }
}

#[derive(Deserialize)]
struct UpdateOrderParam0 {
    id: String,
    // status: i32,
    ty: String,
    receipt_account: String,
    payment_method: String,
    // status = 0
    product: Product,
    customer: Customer,
}
#[derive(Deserialize)]
struct UpdateOrderParam1 {
    id: String,
    // status: i32,
    ty: String,
    receipt_account: String,
    payment_method: String,
    repayment: Repayment,
    #[serde(deserialize_with = "crate::libs::dser::deser_yyyy_mm_dd_hh_mm_ss")]
    transaction_date: String,
    invoice: Invoice,
    ship: Ship,
}

async fn update_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{user} 请求更新订单");
    commit_or_rollback!(async __update_order, &mut conn, value, &user)?;
    log!("{user} 成功更新订单");
    ORDER_CACHE.clear();
    Ok(Response::ok(json!("更新订单成功")))
}

async fn __update_order(conn: &mut PooledConn, value: Value, user: &User) -> Result<(), Response> {
    let Some(id) = op::catch!(value.as_object()?.get("id")?.as_str()) else {
        return Err(Response::invalid_format("缺少id"));
    };
    let mut order = query_order_by_id(conn, id)?;
    if order.salesman.id != user.id {
        log!(
            "用户{}无法修改{}-{}的订单",
            user,
            order.salesman.id,
            order.salesman.name
        );
        return Err(Response::permission_denied());
    }
    if order.status == 0 {
        let mut param: UpdateOrderParam0 = serde_json::from_value(value)?;
        update_status0(conn, &mut param)
    } else if order.status == 1 {
        let mut param: UpdateOrderParam1 = serde_json::from_value(value)?;
        update_status1(conn, user, &mut param, &mut order)
    } else {
        log!(
            "系统拒绝{}修改订单{}的状态，因为该订单处于已完成状态",
            user,
            id
        );
        Err(Response::dissatisfy("该订单处于已完成状态, 不允许被修改"))
    }
}

fn update_status0(conn: &mut PooledConn, param: &mut UpdateOrderParam0) -> Result<(), Response> {
    param.product.query_price(conn, 0, &param.id)?;
    conn.exec_drop(
        "update order_data set ty=:ty, receipt_account=:ra, payment_method=:pm, 
        customer=:customer, address=:address,purchase_unit=:purchase_unit,
        product=:product, discount=:discount, amount=:amount, pre_price=:price
        where id=:id limit 1
     ",
        params! {
            "ty" => &param.ty,
            "ra" => &param.receipt_account,
            "pm" => &param.payment_method,
            "id" => &param.id,
            "customer" => &param.customer.id,
            "address" => &param.customer.address,
            "purchase_unit" => &param.customer.purchase_unit,
            "product" => &param.product.id,
            "discount" => &param.product.discount,
            "amount" => &param.product.amount,
            "price" => &param.product.price

        },
    )?;

    // conn.exec_drop(
    //     "update order_data set customer=?, address=?, purchase_unit=? where id = ? limit 1",
    //     (
    //         &param.customer.id,
    //         &param.customer.address,
    //         &param.customer.purchase_unit,
    //         &param.id,
    //     ),
    // )?;

    // conn.exec_drop(
    //     "update order_data set product=?, discount=?, amount=? where id = ? limit 1",
    //     (
    //         &param.product.id,
    //         &param.product.discount,
    //         &param.product.amount,
    //         &param.id,
    //     ),
    // )?;
    Ok(())
}

fn update_status1(
    conn: &mut PooledConn,
    user: &User,
    param: &mut UpdateOrderParam1,
    order: &mut Order,
) -> Result<(), Response> {
    order.product.query_price(conn, order.status, &order.id)?;

    if param.ship.shipped && (param.ship.date.is_none() || param.ship.storehouse.is_none()) {
        log!(
            "系统拒绝{}修改订单{}的状态，当设置成发货状态时，date和storehouse必须设置",
            user,
            param.id
        );
        return Err(Response::dissatisfy("ship的date和storehouse必须设置"));
    }
    let already_finish = order.repayment.instalment.iter().any(|v| v.finish);
    if !already_finish {
        if param.repayment.is_invalid() || !param.repayment.date_is_valid() {
            return Err(Response::invalid_value("回款数据错误"));
        } else if order.product.price_sum_with_discount() != param.repayment.sum() {
            println!(
                "{}  {}",
                order.product.price_sum_with_discount(),
                param.repayment.sum()
            );
            return Err(Response::invalid_value("回款金额错误"));
        }
        conn.exec_drop(
            "update order_data set repayment_model = ? where id = ? limit 1",
            (param.repayment.model, &param.id),
        )?;
        conn.exec_drop(
            "delete from order_instalment where order_id = ?",
            (&param.id,),
        )?;
        for item in &mut order.repayment.instalment {
            item.finish = order.status == 2;
        }
        param.repayment.smart_insert(&param.id, conn)?;
    }
    conn.exec_drop(
        "update order_data set ty=:ty, receipt_account=:ra, payment_method=:pm, invoice_required=:ir, 
            shipped=:ship, shipped_date=:date, shipped_storehouse=:storehouse, transaction_date=:trdate
            where id=:id limit 1
     ",
        params! {
            "ty" => &param.ty,
            "ra" => &param.receipt_account,
            "pm" => &param.payment_method,
            "trdate" => &param.transaction_date,
            "id" => &param.id,
            "ir" => &param.invoice.required,
            "ship" => &param.ship.shipped,
            "storehouse" => &param.ship.storehouse,
            "date" => &param.ship.date
        },
    )?;

    if param.invoice.required {
        if !order.invoice.required {
            let number = gen_number!(
                conn,
                1,
                format!("INV{}{}", order.salesman.name(), order.customer.name)
            );
            param.invoice.number = number;
            param.invoice.insert(&param.id, conn)?;
        } else {
            conn.exec_drop(
                "update invoice set title=:title, deadline=:dl, description=:desc 
                where order_id = :id",
                params! {
                    "id" => &param.id,
                    "title" => &param.invoice.title,
                    "dl" => &param.invoice.deadline,
                    "desc" => &param.invoice.description
                },
            )?;
        }
    } else {
        conn.exec_drop(
            "delete from invoice where order_id = ? limit 1",
            (&param.id,),
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
        c.company, p.name as product_name, p.unit,
        p.cover as product_cover
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
    #[serde(default)]
    limit: u32,
    status: i32,
}

async fn query_person_order<'err>(
    conn: &mut DB<'err>,
    param: &QueryParams,
    user: &User,
    status: &str,
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
        format!(
            "select o.*, u.name as salesman_name, c.name as customer_name, 
        c.company, p.name as product_name, p.price as product_price, p.unit,
            p.cover as product_cover
        from order_data o
        join user u on u.id = o.salesman
        join customer c on c.id = o.customer
        join product p on p.id = o.product
        where o.salesman = ? and o.status {status}
        order by o.create_time desc
        limit ?
    "
        ),
        (&id, &param.limit),
    )
    .map_err(Into::into)
}

async fn query_department_order<'err>(
    conn: &mut DB<'err>,
    param: &QueryParams,
    user: &User,
    status: &str,
) -> Result<Vec<Order>, Response> {
    if !verify_perms!(&user.role, OtherGroup::NAME, OtherGroup::QUERY_ORDER) {
        return Err(Response::permission_denied());
    }
    dbg!("部门");
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
        format!(
            "select o.*, u.name as salesman_name, c.name as customer_name, 
        c.company, p.name as product_name, p.unit, p.cover as product_cover
        from order_data o
        join user u on u.id = o.salesman and u.department = ?
        join customer c on c.id = o.customer
        join product p on p.id = o.product
        where o.status {status}
        order by o.create_time desc
        limit ?
    "
        ),
        (&depart, &param.limit),
    )
    .map_err(Into::into)
}

async fn query_company_order(
    conn: &mut PooledConn,
    user: &User,
    limit: u32,
    status: &str,
) -> Result<Vec<Order>, Response> {
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
    conn.query(format!(
        "select o.*,
            u.name as salesman_name,
            c.name as customer_name,
            c.company,
            p.name as product_name,
            p.unit,
            p.cover as product_cover
        from
            order_data o
            join user u on u.id = o.salesman
            join customer c on c.id = o.customer
            join product p on p.id = o.product
        where o.status {status}
        order by
            o.create_time desc
        limit {limit}
            "
    ))
    .map_err(Into::into)
}

async fn query_order(header: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{}-{} 请求查询订单", user.department, user.name);
    let param_str = value.to_string();
    let mut param: QueryParams = serde_json::from_value(value)?;
    if param.limit == 0 {
        param.limit = 50
    }
    let status = if param.status >= 3 {
        ">= 0".to_string()
    } else {
        format!("={}", param.status)
    };
    let value = if let Some(value) = get_cache!(ORDER_CACHE, &uid, &param_str) {
        log!("缓存命中");
        value
    } else {
        log!("缓存未命中11");
        let mut data = match param.ty {
            0 => query_person_order(&mut conn, &param, &user, &status).await?,
            1 => query_department_order(&mut conn, &param, &user, &status).await?,
            2 => query_company_order(&mut conn, &user, param.limit, &status).await?,
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
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    let param: PayParam = serde_json::from_value(value)?;
    log!(
        "{} 请求完成订单{} -  回款日期为{} 的收款",
        user,
        param.id,
        param.date
    );
    let time = TIME::now()?;
    let query = format!(
        "select date from order_instalment where order_id = '{}' and date = '{}'",
        param.id, param.date
    );
    let key: Option<String> = conn.query_first(query)?;
    if key.is_none() {
        log!(
            "收款失败，无法找到该分期回款：ID: {}, date: {} ",
            param.id,
            param.date
        );
        return Err(Response::not_exist("无法找到该分期回款"));
    }
    conn.exec_drop(
        "update order_instalment set finish = 1, finish_time= ? where order_id= ? and date = ? limit 1",
        (time.format(TimeFormat::YYYYMMDD_HHMMSS), &param.id, &param.date),
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

async fn delete_order(header: HeaderMap, Path(id): Path<String>) -> ResponseResult {
    let bearer = bearer!(&header);
    let mut conn = DBC.lock().await;
    let uid = parse_jwt_macro!(&bearer, &mut conn => true);
    let user = get_user(&uid, &mut conn).await?;
    log!("{} 请求删除订单{}", user, id);
    let Some::<Option<String>>(file) = conn.exec_first(
        "select file from order_data where id = ? and salesman = ? limit 1",
        vec![&id, &user.id],
    )?
    else {
        log!("订单不存在或权限不足");
        return Err(Response::permission_denied());
    };
    commit_or_rollback!(__delete_order, &mut conn, &id, &file)?;
    ORDER_CACHE.clear();
    log!("{} 成功删除订单{}", user, id);
    Ok(Response::ok(json!("删除订单成功")))
}

fn __delete_order(
    conn: &mut PooledConn,
    order_id: &str,
    file: &Option<String>,
) -> Result<(), Response> {
    conn.exec_drop(
        "delete from invoice where order_id = ? limit 1",
        (&order_id,),
    )?;
    conn.exec_drop(
        "delete from order_instalment where order_id = ?",
        (&order_id,),
    )?;
    conn.exec_drop("delete from order_data where id = ?", (&order_id,))?;
    if let Some(f) = file {
        std::fs::remove_file(format!("resources/order/{f}"))?;
    }
    Ok(())
}

async fn get_order_file(
    Path(url): Path<String>,
) -> Result<BodyFile, (axum::http::StatusCode, String)> {
    BodyFile::new_with_base64_url("resources/order", &url)
}
