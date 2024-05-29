use mysql::{params, prelude::Queryable, PooledConn};
use mysql_common::prelude::FromRow;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, FromRow, Default)]
pub struct Invoice {
    #[serde(deserialize_with = "crate::libs::deserialize_any_to_bool")]
    pub required: bool,
    pub deadline: String,
    pub title: String,
    pub number: String,
    pub description: String,
}

impl Invoice {
    pub fn update(&self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        conn.exec_drop(
            "update invoice set  title=:title, deadline=:dl,
        description=:d where order_id=:id and number = :num limit 1",
            params! {
                "title" => &self.title,
                "dl" => &self.deadline,
                "d" => &self.description,
                "id" => id,
                "num" => &self.number
            },
        )
    }
    
    pub fn delete(&self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        if id.is_empty() {
            return Ok(());
        }
        conn.exec_drop("delete from invoice where order_id=? and number=? limit 1", (id, &self.number))
    }
    pub fn insert(&self, id: &str, conn: &mut PooledConn) -> mysql::Result<()> {
        conn.exec_drop(
            "insert into  invoice (order_id, number, title, deadline, description)
                values (:id, :num, :title, :dl, :d)",
            params! {
                "num" => &self.number,
                "title" => &self.title,
                "dl" => &self.deadline,
                "d" => &self.description,
                "id" => id
            },
        )
    }
}
