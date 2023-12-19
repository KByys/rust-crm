use crm_rust::database::get_conn;
use mysql::prelude::Queryable;
use mysql_common::prelude::FromRow;

#[derive(FromRow, Debug)]
struct Test {
    id: i32
}

fn main() -> mysql::Result<()> {
    std::fs::write("te/good", b"Hello, world!")?;
    Ok(())
}