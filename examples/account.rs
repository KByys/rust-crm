use crm_rust::database::get_conn;


fn main() -> mysql::Result<()> {
    let mut conn = get_conn()?;
    

    Ok(())
}