use crate::{
    libs::{
        dser::{deser_f32, deser_yyyy_mm_dd, serialize_f32_to_string},
        TIME,
    },
    log,
};
use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default, PartialEq)]
pub struct Repayment {
    pub model: i32,
    pub instalment: Vec<Instalment>,
}
impl Repayment {
    pub fn smart_query(&mut self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        self.instalment = Instalment::query(conn, id)?;
        Ok(())
    }
    pub fn is_invalid(&self) -> bool {
        self.instalment.is_empty() || {
            let flag = self.model == 0 && self.instalment.len() != 1;
            if flag {
                log!("全款有且只能有一期付款")
            }
            flag
        }
    }
    pub fn date_is_valid(&self) -> bool {

        if self.instalment.len() > 1 {
            let mut end = 1;
            let mut start = 0;
            while end < self.instalment.len() {
                if self.instalment[start].date >= self.instalment[end].date {
                    log!("后一个回款日期必须大于之前的");
                    return false;
                }
                start += 1;
                end += 1;
            }
        }
        true
    }
    pub fn smart_insert(&mut self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        Instalment::insert(conn, id, &self.instalment)?;
        Ok(())
    }
    pub fn sum(&self) -> f32 {
        self.instalment.iter().map(|v| v.original_amount).sum()
    }
}

#[derive(Deserialize, FromRow, Serialize, PartialEq)]
pub struct Instalment {
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub interest: f32,
    #[serde(deserialize_with = "deser_f32")]
    #[serde(serialize_with = "serialize_f32_to_string")]
    pub original_amount: f32,
    #[serde(deserialize_with = "deser_yyyy_mm_dd")]
    pub date: String,
    #[serde(deserialize_with = "crate::libs::deserialize_any_to_bool")]
    pub finish: bool,
    #[serde(skip_deserializing)]
    pub finish_time: Option<String>,
}
impl Instalment {
    pub fn query(conn: &mut PooledConn, id: &str) -> mysql::Result<Vec<Instalment>> {
        conn.exec(
            "select * from order_instalment where order_id = ? order by date",
            (id,),
        )
    }
    pub fn insert(conn: &mut PooledConn, id: &str, instalment: &[Instalment]) -> mysql::Result<()> {
        let time = TIME::now().unwrap_or_default();
        conn.exec_batch(
            "insert into order_instalment 
            (order_id, interest, original_amount, date, finish, finish_time) 
            values 
            (:order_id, :interest, :original_amount, :date, :finish, :finish_time) 
            ",
            instalment.iter().map(|v| {
                params! {
                    "order_id" => id,
                    "interest" => v.interest,
                    "original_amount" => v.original_amount,
                    "date" => &v.date,
                    "finish" => v.finish,
                    "finish_time" => if v.finish { 
                        mysql::Value::Bytes(time.format(crate::libs::TimeFormat::YYYYMMDD_HHMMSS).into_bytes()) 
                    } else {
                        mysql::Value::NULL
                    }
                }
            }),
        )
    }
}
