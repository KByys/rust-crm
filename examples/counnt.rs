use std::collections::HashMap;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
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
    let mut tasks = Vec::new();
    for i in 0..100 {
        tasks.push(task::spawn(run(i % 20)));
    }
    for task in tasks {
        task.await.unwrap();
    }
    sleep(Duration::from_secs(6))
}

async fn run(i: i32) {
    if let Some(v) = USER.get(&i) {
        println!("读取{i}成功，内容是：{}", v.read().await)
    } else {
        USER.insert(i, RwLock::new(format!("{i} ------")));
        println!("写入{i}")
    }
}
