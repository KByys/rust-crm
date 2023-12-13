use std::fmt::Display;

use axum::{http::StatusCode, Json};
use serde::{ser::SerializeStruct, Serialize};
use serde_json::{json, Value};
/// 响应数据
#[derive(Debug)]
pub struct Response {
    /// 响应状态码
    code: StatusCode,
    status: i32,
    data: Value,
}

impl axum::response::IntoResponse for Response {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut s = serializer.serialize_struct("Response", 2)?;
        s.serialize_field("status", &self.status)?;
        s.serialize_field("code", &self.code.as_u16())?;
        s.serialize_field("data", &self.data)?;
        s.end()
    }
}
impl Response {
    pub fn new(code: StatusCode, status: i32, data: Value) -> Response {
        Self { code, status, data }
    }
    pub fn ok(data: Value) -> Self {
        Self {
            code: StatusCode::OK,
            status: 0,
            data,
        }
    }
    pub fn empty() -> Self {
        Self {
            code: StatusCode::OK,
            status: 0,
            data: json!("OK"),
        }
    }
    pub fn token_error(e: impl Display) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, -1, json!(e.to_string()))
    }
    /// 内部错误
    pub fn internal_server_error(e: impl Display) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, -1, json!(e.to_string()))
    }
    /// 参数格式错误
    pub fn invalid_format(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 1, json!(e.to_string()))
    }
    /// 请求的数据不存在
    pub fn not_exist(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 2, json!(e.to_string()))
    }
    /// 要添加的数据已存在
    pub fn already_exist(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 3, json!(e.to_string()))
    }
    /// 权限不足
    pub fn permission_denied() -> Self {
        Self::new(StatusCode::OK, 4, json!("权限不足"))
    }
    /// 密码错误
    pub fn wrong_password() -> Self {
        Self::new(StatusCode::OK, 5, json!("密码错误"))
    }
    /// 用于测试期间暂时不支持的功能
    pub fn not_supported() -> Self {
        Self::new(StatusCode::OK, 6, json!("暂时不支持"))
    }
    /// 数值不对
    pub fn invalid_value(e: impl Display) -> Self {
        Self::new(StatusCode::OK, 7, json!(e.to_string()))
    }
    pub fn code(&self) -> StatusCode {
        self.code
    }
    pub fn status(&self) -> i32 {
        self.status
    }
}

impl From<mysql::Error> for Response {
    fn from(value: mysql::Error) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            -1,
            json!(value.to_string()),
        )
    }
}
impl From<std::io::Error> for Response {
    fn from(value: std::io::Error) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            -1,
            json!(value.to_string()),
        )
    }
}

impl From<serde_json::Error> for Response {
    fn from(value: serde_json::Error) -> Self {
        Response::invalid_format(value)
    }
}
impl From<std::time::SystemTimeError> for Response {
    fn from(value: std::time::SystemTimeError) -> Self {
        Response::internal_server_error(value)
    }
}
