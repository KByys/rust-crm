use std::net::{Ipv4Addr, SocketAddrV4};

use axum::{extract::DefaultBodyLimit, http::Method, Router};
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

    let port = match std::fs::read_to_string("port") {
        Ok(port) => port.parse::<u16>().expect("端口号错误"),
        _ => {
            std::fs::write("port", "80").expect("创建port文件失败，请手动创建");
            panic!("读取端口失败, 请在port文件中写入端口号，例如3389, 443, 80")
        }
    };
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    axum::serve(tokio::net::TcpListener::bind(&addr).await.unwrap(), router)
        .await
        .unwrap()
}
