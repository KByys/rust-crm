use mysql::{params, prelude::Queryable, Pool};
use op::ternary;
fn main() -> mysql::Result<()> {
    let pool = Pool::new("mysql://new_user:password@localhost:3306/crm1")?;
    let mut conn = pool.get_conn()?;
    // let s = include_str!("re.sql");
    // for s in s.split(';') {
    //     println!("{s}--------------");
        
    // }
    let s: Option<String> = conn.query_first("SELECT 1 FROM user")?;
        println!("{:?}", s);
    // conn.query_drop(s)?;
    // let ter = conn.query_first::<String, &str>("SELECT role FROM user");
    // println!("{:?}", ter);
    //  conn.exec_drop(
    //     "INSERT INTO user (id, password, name,  sex, role, department) VALUES (
    //     :id, :password, :name, :sex, :role, :department
    // )",
    //     params! {
    //         "id" => "223456",
    //         "password" => md5::compute("12345678").0,
    //         "name" => "345643",
    //         "sex" => 0,
    //         "role" => "root",
    //         "department" => "总经办",
    //     },
    // )?;
    Ok(())
}
