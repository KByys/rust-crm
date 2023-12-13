use chrono::prelude::TimeZone;
use crm_rust::libs::time::*;
fn main() {
    let time = TIME::now().unwrap();
    println!("{}", time.format(TimeFormat::YYYYMMDD) < time.format(TimeFormat::YYYYMMDD_HHMMSS))
}