use std::{
    thread::sleep,
    time::Duration,
};

use crm_rust::database::get_conn;
use mysql::prelude::Queryable;

fn main() -> mysql::Result<()> {
    let mut conn = get_conn()?;
    for i in 0..10 {
        conn.query_drop("BEGIN")?;
        conn.query_drop(format!("INSERT INTO stu VALUES ('{}', '3455')", i))?;
        if i % 2 == 0 {
            conn.query_drop("COMMIT")?;
        } else {
            sleep(Duration::from_secs(1));
            conn.query_drop("ROLLBACK")?;
        }
        println!("{}", i);
    }
    Ok(())
}
