use axum::{
    extract::{DefaultBodyLimit, Path},
    http::{Method, StatusCode},
    routing::{get, post},
    Router,
};
use crm_rust::verify_perms;

#[tokio::main]
async fn main() {
    let sql = include_str!("../src/database/table.sql");
    for s in sql.split(';') {
        println!("{s}-------------")
    }
}
