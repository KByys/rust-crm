use crate::libs::dser::{deserialize_storehouse, op_deser_yyyy_mm_dd_hh_mm_ss};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Ship {
    pub shipped: bool,
    #[serde(deserialize_with = "op_deser_yyyy_mm_dd_hh_mm_ss")]
    pub date: Option<String>,
    #[serde(deserialize_with = "deserialize_storehouse")]
    pub storehouse: String,
}
