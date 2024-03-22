use crate::{
    libs::dser::{deserialize_storehouse, serialize_f32_to_string},
    Response,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Deserializer, Serialize};

pub fn deserialize_f32_max_1<'de, D>(de: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: String = Deserialize::deserialize(de)?;
    if let Ok(f) = value.parse::<f32>() {
        op::ternary!(f <= 1.0 => Ok(f), Err(serde::de::Error::custom("discount最大值为1")))
    } else {
        Err(serde::de::Error::custom("price不是浮点数格式"))
    }
}
#[derive(Deserialize, Serialize)]
pub struct Product {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "deserialize_f32_max_1")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub discount: f32,
    pub inventory: Vec<Inventory>,
}

impl Product {
    pub fn price(&self, conn: &mut PooledConn) -> Result<f32, Response> {
        conn.exec_first(
            "select price from product where id = ? limit 1",
            (&self.id,),
        )
        .map_err(Into::into)
        .and_then(|f| {
            if let Some(f) = f {
                Ok(f)
            } else {
                Err(Response::not_exist("产品不存在"))
            }
        })
    }
    pub fn price_sum(&self, price: f32) -> f32 {
        self.inventory.iter().map(|v| v.amount as f32 * price).sum()
    }

    pub fn price_sum_with_discount(&self, price: f32) -> f32 {
        self.price_sum(price) * self.discount
    }
    pub fn query_inventory(&mut self, order: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        self.inventory = conn.exec(
            "select storehouse, amount 
                from order_product_inventory 
                where order_id = ? 
                order by storehouse",
            (order,),
        )?;

        Ok(())
    }
    pub fn add_inventory(&self, conn: &mut PooledConn, order: &str) -> mysql::Result<()> {
        conn.exec_batch(
            "insert into order_product_inventory 
        (order_id, amount, storehouse)
        values
        (:order_id, :amount, :storehouse)
        ",
            self.inventory.iter().map(|v| {
                params! {
                    "order_id" => order,
                    "amount" => v.amount,
                    "storehouse" => &v.storehouse
                }
            }),
        )
    }
}

#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Inventory {
    #[serde(deserialize_with = "deserialize_storehouse")]
    pub storehouse: String,
    pub amount: i32,
}
