use crate::libs::dser::{deser_f32, deser_yyyy_mm_dd, serialize_f32_to_string};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Repayment {
    pub model: i32,
    pub instalment: Vec<Instalment>,
}
impl Repayment {
    pub fn smart_query(&mut self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        if self.model != 0 {
            self.instalment = Instalment::query(conn, id)?;
        }
        Ok(())
    }

    pub fn smart_insert(&mut self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        if self.model != 0 {
            Instalment::insert(conn, id, &self.instalment)?;
        }
        Ok(())
    }
    pub fn sum(&self) -> Option<f32> {
        if self.model != 0 {
            Some(self.instalment.iter().map(|v| v.original_amount).sum())
        } else {
            None
        }
    }
}

#[derive(Deserialize, FromRow, Serialize)]
pub struct Instalment {
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub interest: f32,
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub original_amount: f32,
    #[serde(deserialize_with = "deser_yyyy_mm_dd")]
    pub date: String,
    pub finish: bool,
}
impl Instalment {
    pub fn query(conn: &mut PooledConn, id: &str) -> mysql::Result<Vec<Instalment>> {
        conn.exec(
            "select * from order_instalment where order_id = ? order by date",
            (id,),
        )
    }
    pub fn insert(conn: &mut PooledConn, id: &str, instalment: &[Instalment]) -> mysql::Result<()> {
        conn.exec_batch(
            "insert into order_instalment 
            (order_id, interest, original_amount, date, finish) 
            values 
            (:order_id, :interest, :original_amount, :date, :finish) 
            ",
            instalment.iter().map(|v| {
                params! {
                    "order_id" => id,
                    "interest" => v.interest,
                    "original_amount" => v.original_amount,
                    "date" => &v.date,
                    "finish" => v.finish
                }
            }),
        )
    }
}
