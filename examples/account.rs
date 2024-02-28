use axum::{
    extract::{DefaultBodyLimit, Path},
    http::{Method, StatusCode},
    routing::{get, post},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

async fn hello(Path(message): Path<String>) -> (StatusCode, String) {
    println!("接收到的信息{}", message);
    (StatusCode::OK, "收到了".to_string())
}

#[tokio::main]
async fn main() {
    let router = Router::new()
        .route("/hello/:message", get(hello))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods([Method::GET, Method::POST, Method::DELETE])
                .allow_headers(Any),
        )
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024));
    axum::serve(
        tokio::net::TcpListener::bind(format!("0.0.0.0:{}", 8888))
            .await
            .unwrap(),
        router,
    )
    .await
    .unwrap()
}
