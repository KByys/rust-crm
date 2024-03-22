use mysql::{params, prelude::Queryable, Pool};
use op::ternary;
fn main() -> mysql::Result<()> {
    let pool = Pool::new("mysql://root:313aaa@localhost:3306/crm")?;
    let mut conn = pool.get_conn()?;
    // let s = include_str!("re.sql");
    // for s in s.split(';') {
    //     println!("{s}--------------");

    // }
    conn.query_drop("create table if not exists testt (name varchar(10), te varchar(10))")?;
    conn.exec_batch("insert into testt (name, te) values (?, ?)", ["12", "23"].iter().map(|v|("444", v)))?;
    Ok(())
}
