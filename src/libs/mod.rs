pub mod dser;
pub mod headers;
pub mod perm;
pub mod time;


use base64::prelude::Engine;
/// base64 url safe encode
pub fn base64_encode(input: impl AsRef<[u8]>) -> String {
    base64::prelude::BASE64_URL_SAFE_NO_PAD.encode(input)
}

/// base64 url safe decode
pub fn base64_decode(input: impl AsRef<[u8]>) -> Result<Vec<u8>, base64::DecodeError> {
    base64::prelude::BASE64_URL_SAFE_NO_PAD.decode(input)
}

