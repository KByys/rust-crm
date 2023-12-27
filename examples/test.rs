
use std::{thread::sleep, time::Duration};

use crm_rust::{database::get_conn, base64_encode, libs::time::TIME};
use mysql::prelude::Queryable;
#[tokio::main]
async fn main() -> mysql::Result<()> {
     let time = TIME::now().unwrap();
     
    let id = base64_encode(format!(
        "{}-{}-{}",
        "你好的十多个冻死",
        time.naos() / 10000,
        rand::random::<u8>()
    ));
    println!("{}", id.len());
    Ok(())
}

async fn run(size: usize) -> mysql::Result<()> {

    let mut conn = get_conn()?;
    for i in size..size + 9 {
        conn.query_drop("BEGIN")?;
        conn.query_drop(format!("INSERT INTO stu VALUES ('{}', '3455')", i))?;
        if i % 2 == 0 {
            conn.query_drop("COMMIT")?;
        } else {
            tokio::time::sleep(Duration::from_millis(10)).await;
            conn.query_drop("ROLLBACK")?;
        }
        println!("{}", i);
    }
    Ok(())
}
