use crm_rust::database::get_conn;
use mysql::prelude::Queryable;
use mysql_common::prelude::FromRow;

#[derive(FromRow, Debug)]
struct Test {
    id: i32
}

fn main() -> mysql::Result<()> {
    let mut conn = get_conn()?;
    let sd: Test = conn.query_first("SELECT * FROM test1")?.unwrap();
    println!(
        "{:?}", sd
    );
    Ok(())
}