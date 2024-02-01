
use crm_rust::pages::CUSTOM_FIELD_INFOS;
use mysql::{params, prelude::Queryable, Pool};
use op::ternary;
fn main() -> mysql::Result<()> {
    let pool = Pool::new("mysql://new_user:password@localhost:3306/crm")?;
    let mut conn = pool.get_conn()?;
    let ter = conn.query_first::<String, &str>("SELECT role FROM user");
    println!("{:?}", ter);
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
/// ret => return
/// 
/// con => continue
macro_rules! some {
    
    ($arg:expr; ret) => {
        if let Some(value) = $arg {
            value
        } else {
            return;
        }

    };
    ($arg:expr; ret $result:expr) => {
        if let Some(value) = $arg {
            value
        } else {
            return $result;
        }

    };
    ($arg:expr; con) => {
        if let Some(v) = $arg {
            v
        } else {
            continue;
        }
    };
    
    ($arg:expr; break) => {
        if let Some(v) = $arg {
            v
        } else {
            break;
        }
    };
    ($arg:expr; break $end:expr) => {
        if let Some(v) = $arg {
            v
        } else {
            break $end;
        }
    };
}
#[test]
fn test() {
    for i in 0..5 {
        let so = Some(i);
        let none: Option<i32> = None;
        println!("{}", some!(ternary!(i % 2 == 0 => so; none); ret))
    }
}