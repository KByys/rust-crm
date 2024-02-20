use lazy_static::lazy_static;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

lazy_static! {
    static ref MY_HASHMAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();
        m.insert("key1", "value1");
        m.insert("key2", "value2");
        m.insert("key3", "value3");
        m
    };
}
#[derive(Debug, Deserialize)]
struct Test {
    key1: String,
    key2: String,
    key3: String,
}

fn main() {

    let value: Value = serde_json::from_str(&format!("{:?}", MY_HASHMAP.clone())).unwrap();
    println!("{:#?}", value);
    println!("{:#?}", serde_json::from_value::<HashMap<String, String>>(value.clone()));
    println!("{:#?}", serde_json::from_value::<Test>(value.clone()));
}
