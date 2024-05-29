use crate::{libs::dser::serialize_f32_to_string, Response};
use mysql::{prelude::Queryable, PooledConn};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub fn deserialize_f32_max_1<'de, D>(de: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Value = Deserialize::deserialize(de)?;
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                Ok(f as f32)
            } else if let Some(i) = n.as_i64() {
                Ok(i as f32)
            } else if let Some(u) = n.as_u64() {
                Ok(u as f32)
            } else {
                Err(serde::de::Error::custom("discount不是浮点数格式"))
            }
        }
        Value::String(value) => {
            if let Ok(f) = value.parse::<f32>() {
                op::ternary!(f <= 1.0 => Ok(f), Err(serde::de::Error::custom("discount最大值为1")))
            } else {
                Err(serde::de::Error::custom("discount不是浮点数格式"))
            }
        }
        _ => Err(serde::de::Error::custom("discount不是浮点数格式")),
    }
}
#[derive(Deserialize, Serialize, Debug)]
pub struct Product {
    pub id: String,
    pub name: String,
    #[serde(deserialize_with = "deserialize_f32_max_1")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub discount: f32,
    #[serde(skip_deserializing)]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub price: f32,
    #[serde(skip_deserializing)]
    pub cover: String,
    pub amount: usize,
    #[serde(skip_deserializing)]
    pub unit: String,
}

impl Product {
    pub fn query_price(
        &mut self,
        conn: &mut PooledConn,
        status: i32,
        order_id: &str,
    ) -> Result<(), Response> {
        if status == 0 {
            conn.exec_first(
                "select price from product where id = ? limit 1",
                (&self.id,),
            )
            .map_err(Into::into)
            .and_then(|f| {
                if let Some(f) = f {
                    self.price = f;
                    Ok(())
                } else {
                    Err(Response::not_exist("产品不存在"))
                }
            })
        } else {
            conn.exec_first(
                "select pre_price from order_data where id = ? limit 1",
                (order_id,),
            )
            .map_err(Into::into)
            .and_then(|f| {
                if let Some(f) = f {
                    self.price = f;
                    Ok(())
                } else {
                    Err(Response::not_exist("订单不存在"))
                }
            })
        }
    }
    pub fn price_sum(&self) -> f32 {
        self.amount as f32 * self.price
    }

    pub fn price_sum_with_discount(&self) -> f32 {
        let sum = self.price_sum();
        sum - self.discount * sum
    }
}
