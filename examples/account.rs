use crm_rust::{database::get_conn, do_if};
use mysql::prelude::Queryable;
use mysql_common::prelude::FromRow;
use serde_json::json;

#[derive(FromRow, Debug)]
struct Test {
    id: i32
}

fn main() -> mysql::Result<()> {
    let d = 1;
    println!("{}", do_if!(d == 1 => 1, 0));
    Ok(())
}