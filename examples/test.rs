use std::path::PathBuf;

use axum::{
    extract::{DefaultBodyLimit, Multipart},
    http::{Method, StatusCode},
    routing::{get, post},
    Router,
};
use crm_rust::{libs::parse_multipart, response::BodyFile, Response, ResponseResult};
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    let ver_path = PathBuf::from("version");
    if !ver_path.exists() {
        std::fs::write("version", "0.0.0").unwrap();
    }
    let router = Router::new()
        .route("/upload/server", post(upload_server))
        .route("/version", get(get_version))
        .route("/latest/server", get(get_latest_server))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(Any),
        )
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024));

    let tcp = TcpListener::bind("0.0.0.0:80").await.unwrap();
    axum::serve(tcp, router).await.unwrap();
}
async fn get_version() -> (StatusCode, String) {
    let version = std::fs::read_to_string("version").unwrap_or(String::from("0.0.1"));
    println!("最新版本: {version}");
    (StatusCode::OK, version)
}
async fn get_latest_server() -> Result<BodyFile, (StatusCode, String)> {
    let server = std::fs::read("server.exe").unwrap();
    Ok(BodyFile::new(server))
}
async fn upload_server(part: Multipart) -> ResponseResult {
    let data = parse_multipart(part).await?;
    let version = data.json;
    std::fs::write("version", version)?;
    let server = &data.files[0];
    std::fs::write("server.exe", &server.bytes)?;
    Ok(Response::empty())
}
