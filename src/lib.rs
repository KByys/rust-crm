pub mod response;
pub mod token;

pub mod database;
pub mod libs;
pub mod pages;

use chrono::prelude::TimeZone;
use libs::time::TIME;
pub use libs::{base64_decode, base64_encode};
use mysql_common::prelude::FromRow;
pub use response::Response;

pub type ResponseResult = Result<Response, Response>;
#[inline]
pub fn debug_info(info: String) {
    let time = TIME::now().unwrap_or_default().naos();
    println!(
        "{} -- {}",
        chrono::Local.timestamp_nanos(time as i64).to_rfc3339(),
        info
    )
}

#[derive(serde::Serialize, FromRow, Debug, serde::Deserialize, Clone)]
pub struct TextInfos {
    pub display: String,
    pub value: String,
}



