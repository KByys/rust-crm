use std::collections::HashMap;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use serde_json::json;
use tokio::sync::RwLock;
use tokio::task;
use dashmap::DashMap;
lazy_static::lazy_static! {
    static ref USER: Arc<DashMap<i32, RwLock<String>>> =  {
        Arc::new(DashMap::new())
    };
}

#[tokio::main]
async fn main() {
    let d = Arc::new(34);
    let json = json!(d.as_ref());
    println!("{}", json)
}

async fn run(i: i32) {
    if let Some(v) = USER.get(&i) {
        println!("读取{i}成功，内容是：{}", v.read().await)
    } else {
        USER.insert(i, RwLock::new(format!("{i} ------")));
        println!("写入{i}")
    }
}
