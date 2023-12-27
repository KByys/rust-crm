
use crm_rust::pages::CUSTOM_FIELD_INFOS;
use mysql::{prelude::Queryable, Pool};

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
