use std::collections::HashMap;

use serde_json::json;
use tokio::sync::Mutex;

lazy_static::lazy_static! {
    pub static ref USERS: Mutex<HashMap<&'static str, &'static str>> = {
        let mut map = HashMap::new();
        map.insert("1", "1");
        map.insert("2", "2");
        Mutex::new(map)
    };
}
#[tokio::main]
async fn main() {
    let mut da = USERS.lock().await;
    da.insert("d", "dfs");
    println!("{:?}", da.clone())

}