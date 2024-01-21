use lazy_static::lazy_static;
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

fn main() {
    println!("{:?}", md5::compute(b"12345678").0);
}
