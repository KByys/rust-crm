
use crm_rust::pages::CUSTOM_FIELD_INFOS;
use mysql::{prelude::Queryable, Pool};
use op::ternary;
fn main() -> mysql::Result<()> {
    let pool = Pool::new("mysql://root:313aaa@localhost:3306/crm")?;
    let mut conn = pool.get_conn()?;
    for table in CUSTOM_FIELD_INFOS {
        for t in table {
            conn.query_drop(format!("ALTER table {t} modify column id VARCHAR(150) NOT NULL"))?;
            
        }
    }
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