use std::collections::HashMap;

use mysql::{prelude::FromRow, FromRowError};
use serde::{Deserialize, Serialize};

use crate::{common::Person, libs::dser::deser_empty_to_none};

use super::{
    customer::Customer, invoice::Invoice, payment::Repayment, product::Product, ship::Ship,
};

#[derive(Deserialize, Serialize)]
pub struct Order {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub create_time: String,
    pub number: String,
    pub status: i32,
    pub ty: String,
    #[serde(deserialize_with = "deser_empty_to_none")]
    pub transaction_date: Option<String>,
    pub receipt_account: String,
    pub salesman: Person,
    pub payment_method: String,
    pub repayment: Repayment,
    pub product: Product,
    pub customer: Customer,
    pub invoice: Invoice,
    pub ship: Ship,
}

macro_rules! get {
    ($map:expr, $name:expr) => {{
        mysql::prelude::FromValue::from_value($map.get($name)?.clone())
    }};
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
                model: get!(map, "repayment_model"),
                instalment: Vec::new()
            },
            product: Product {discount:get!(map,"discount"),id:get!(map,"product"),name:get!(map,"product_name"),amount:get!(map,"amount"), price: get!(map, "product_price") },
            customer: Customer {
                id: get!(map, "customer"),
                address: get!(map, "address"),
                name: get!(map, "customer_name"),
                company: get!(map, "company"),
                purchase_unit: get!(map, "purchase_unit")
            },
            invoice: Invoice {
                required: get!(map, "invoice_required"),
                ..Default::default()
            },
            ship: Ship {
                shipped: get!(map, "shipped"),
                date: get!(map, "shipped_date"),
                storehouse: get!(map, "shipped_storehouse")
            },
        }));
        if let Some(order) = result {
            Ok(order)
        } else {
            Err(FromRowError(_row))
        }
    }
}
