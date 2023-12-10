use chrono::prelude::TimeZone;
use crm_rust::libs::time::*;
fn main() {
    let time = TIME::now().unwrap();
    let local = chrono::Local.timestamp_nanos(time.naos() as i64);
    println!("{}", time.naos());
    println!("{:?}", local.timestamp_nanos_opt())
}