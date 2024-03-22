use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, FromRow, Default)]
pub struct Invoice {
   pub required: bool,
   pub deadline: String,
   pub ty: String,
   pub title: String,
   pub number: String,
   pub description: String
}


