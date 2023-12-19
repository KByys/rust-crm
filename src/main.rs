use std::fs::create_dir;

use axum::{extract::DefaultBodyLimit, http::Method, Router};
use crm_rust::{read_data, Config, MYSQL_URI};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crm=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
    _create_all_dir().unwrap();
    read_data();
    let setting = Config::read();
    unsafe {
        MYSQL_URI = setting.mysql_addr();
    }
    crm_rust::database::create_table().unwrap();
    let router = Router::new()
        .merge(crm_rust::pages::pages_router())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(Any),
        )
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024));
    axum::serve(
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", setting.port()))
            .await
            .unwrap(),
        router,
    )
    .await
    .unwrap()
}

fn _create_all_dir() -> std::io::Result<()> {
    _create_dir("config")?;
    _create_dir("data")?;
    Ok(())
}
fn _create_dir(path: &str) -> std::io::Result<()> {
    match create_dir(path) {
        Ok(()) => Ok(()),
        Err(e) => match e.kind() {
            std::io::ErrorKind::AlreadyExists => Ok(()),
            _ => Err(e),
        },
    }
}
