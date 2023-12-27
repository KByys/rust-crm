pub mod dser;
pub mod headers;
pub mod perm;
pub mod time;

use axum::extract::Multipart;
use base64::prelude::Engine;

use crate::Response;

use self::time::TIME;
/// base64 url safe encode
pub fn base64_encode(input: impl AsRef<[u8]>) -> String {
    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(input)
}

/// base64 url safe decode
pub fn base64_decode(input: impl AsRef<[u8]>) -> Result<Vec<u8>, base64::DecodeError> {
    base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(input)
}
/// 三目运算符，用宏简单实现
#[macro_export]
macro_rules! do_if {
    ($pat:expr => $suc:expr, $e:expr) => {
        if $pat {
            $suc
        } else {
            $e
        }
    };
}
pub struct FilePart {
    pub bytes: Vec<u8>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
}
pub struct MessagePart {
    pub files: Vec<FilePart>,
    pub json: String,
}

pub async fn parse_multipart(mut part: Multipart) -> Result<MessagePart, Response> {
    let mut files = Vec::new();
    let mut data = String::new();
    while let Some(field) = part.next_field().await? {
        match field.name() {
            Some("file") => {
                let filename = field.file_name().map(|s| s.to_owned());
                let content_type = field.content_type().map(|s| s.to_string());
                let chunk = field.bytes().await?.to_vec();
                files.push(FilePart {
                    bytes: chunk,
                    filename,
                    content_type,
                });
            }
            Some("data") => {
                data = field.text().await?;
            }
            _ => (),
        }
    }
    Ok(MessagePart { files, json: data })
}

pub fn gen_id(time: &TIME, name: &str) -> String {
    base64_encode(format!(
        "{}-{}-{}",
        name,
        time.naos() / 10000,
        rand::random::<u8>()
    ))
}
