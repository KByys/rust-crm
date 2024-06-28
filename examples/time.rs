use std::{thread::sleep, time::{Duration, SystemTime}};

use crm_rust::libs::TIME;
use regex::Regex;
use serde_json::json;

fn main() {
    let map = crm_rust::libs::perm::default_role_perms();
    std::fs::write("perm.json", json!(map).to_string()).unwrap();
}
