use std::fs::create_dir;

use axum::{extract::DefaultBodyLimit, http::Method, Router};
use crm_rust::{
    database::get_conn,
    pages::{DROP_DOWN_BOX, STATIC_CUSTOM_BOX_OPTIONS, STATIC_CUSTOM_FIELDS},
    perm::roles::ROLE_TABLES,
    read_data, Config, MYSQL_URI,
};
use tower_http::cors::{Any, CorsLayer};
#[tokio::main]
async fn main() {
    _create_all_dir().unwrap();
    read_data();
    let setting = Config::read();
    unsafe {
        MYSQL_URI = setting.mysql_addr();
    }
    crm_rust::database::create_table().unwrap();
    unsafe { init_static() };
    let router = Router::new()
        .merge(crm_rust::pages::pages_router())
        .merge(crm_rust::perm::perm_router())
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
/// 初始化静态数据
unsafe fn init_static() {
    let mut conn = get_conn().expect("初始化失败");
    ROLE_TABLES.init(&mut conn);
    STATIC_CUSTOM_FIELDS
        .init(&mut conn, "custom_fields")
        .expect("err code: 1");
    STATIC_CUSTOM_BOX_OPTIONS
        .init(&mut conn, "custom_field_option")
        .expect("err code: 2");
    DROP_DOWN_BOX.init(&mut conn).expect("err code: 3");
    // crm_rust::pages::func::cust::STATIC_CUSTOMER_LEVEL.init();
}
fn _create_all_dir() -> std::io::Result<()> {
    _create_dir("config")?;
    _create_dir("data")?;
    _create_dir("resources")?;
    _create_dir("resources/product")?;
    _create_dir("resources/product/cover")?;
    _create_dir("resources/approval")?;
    _create_dir("resources/sign")?;
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
