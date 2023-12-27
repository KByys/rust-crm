use crm_rust::database::get_conn;
use mysql::prelude::Queryable;
use mysql_common::prelude::FromRow;
use serde_json::json;

#[derive(FromRow, Debug)]
struct Test {
    id: i32
}

fn main() -> mysql::Result<()> {
    let data = vec!["213", "3水电费交多少", "sdfdsfds"];
    let d = json!(data);
    println!("{}", d);
    let dd: Vec<String> = serde_json::from_str(&d.to_string()).unwrap();
    Ok(())
}