use std::collections::BTreeMap;

use chrono::{prelude::TimeZone, Days};
use hmac::{Hmac, Mac};
use jwt::{Header, SignWithKey, Token, VerifyWithKey};
use mysql::{prelude::Queryable, PooledConn};
use sha2::Sha512;

use crate::libs::{headers::Bearer, time::TIME};
/// 从请求头中获取token
#[macro_export]
macro_rules! bearer {
    // 允许没有token
    ($header:expr, Allow Missing) => {
        match $crate::libs::headers::Bearer::try_from($header) {
            Ok(bearer) => Some(bearer),
            Err(e) => match e {
                $crate::libs::headers::HeaderParserError::MissingHeaderValue(_) => None,
                $crate::libs::headers::HeaderParserError::InvalidValue(_) => None,
                _ => return Err($crate::response::Response::token_error(e)),
            },
        }
    };
    ($header:expr) => {
        match $crate::libs::headers::Bearer::try_from($header) {
            Ok(bearer) => bearer,
            Err(e) => return Err($crate::response::Response::token_error(e)),
        }
    };
}
#[derive(PartialEq, Eq)]
pub enum TokenVerification {
    Ok,
    Expired,
    Error,
}
impl TokenVerification {
    pub fn is_ok(&self) -> bool {
        *self == TokenVerification::Ok
    }
    pub fn is_expired(&self) -> bool {
        *self == TokenVerification::Expired
    }
    pub fn is_error(&self) -> bool {
        *self == TokenVerification::Error
    }
}
static SECRET_KEY: &str = "_SECRET_KEY_RUST_SERVER";
#[derive(Debug, Default)]
pub struct JWToken {
    pub id: String,
    /// true 公司人员，false 客户
    pub sub: bool,
    /// 签发者
    pub iss: String,
    /// 签发时间
    pub iat: i64,
    /// 过期时间
    pub exp: i64,
}
impl JWToken {
    pub fn verify(&self, conn: &mut PooledConn) -> mysql::Result<TokenVerification> {
        // 检查用户是否存在
        let is_exist = if self.sub {
            conn.query_first::<String, String>(format!(
                "SELECT id FROM user WHERE id = '{}'",
                self.id
            ))?
        } else {
            conn.query_first(format!(
                "SELECT id FROM customer_login WHERE id = '{}'",
                self.id
            ))?
        };
        if is_exist.is_none() {
            return Ok(TokenVerification::Error);
        }
        // 检查token的签名时间是否在允许范围内
        let ty = if self.sub { 0 } else { 1 };
        let tbn: Option<i64> = conn.query_first(format!(
            "SELECT tbn FROM token WHERE id = '{}' AND ty = {}",
            self.id, ty
        ))?;
        if let Some(tbn) = tbn {
            let tbn = chrono::Local.timestamp_nanos(tbn);
            let iat = chrono::Local.timestamp_nanos(self.iat);
            if tbn > iat {
                return Ok(TokenVerification::Error);
            }
        }
        // 检查是否过期
        let exp = chrono::Local.timestamp_nanos(self.exp);
        if TIME::now().is_ok_and(|t| t.naos() as i64 >= exp.timestamp_nanos_opt().unwrap_or(0)) {
            return Ok(TokenVerification::Expired);
        }
        Ok(TokenVerification::Ok)
    }
    pub fn is_refresh(&self) -> bool {
        let now = TIME::now().unwrap();
        let now = chrono::Local.timestamp_nanos(now.naos() as i64);
        let exp = chrono::Local.timestamp_nanos(self.exp);
        //过期前一天
        let sub_one_day = exp.checked_sub_days(Days::new(1)).unwrap();
        // 过期后一天
        let add_one_day = exp.checked_add_days(Days::new(1)).unwrap();
        now >= sub_one_day && now <= add_one_day
    }
}
#[macro_export]
macro_rules! parse_jwt_macro {
    ($bearer:expr, $conn:expr => $sub:expr) => {
        match $crate::token::parse_jwt($bearer) {
            Some(jwt) => {
                if jwt.sub == $sub && jwt.verify($conn)?.is_ok() {
                    jwt.id
                } else {
                    return Err($crate::Response::token_error("Invalid Token"));
                }
            }
            _ => return Err($crate::Response::token_error("Invalid Token")),
        }
    }; // 允许token过期
       // ($bearer:expr, $conn:expr, $sub:expr => Allow Expired) => {
       //     match $crate::token::parse_jwt($bearer) {
       //         Some(jwt) => {
       //             if jwt.sub == $sub && {
       //                 let ver = jwt.verify($conn)?;
       //                 ver.is_ok() || ver.is_expired()
       //             } {
       //                 jwt.id
       //             } else {
       //                 return Err($crate::Response::token_error("Invalid Token"));
       //             }
       //         }
       //         _ => return Err($crate::Response::token_error("Invalid Token")),
       //     }
       // };
}

pub fn parse_jwt(bearer: &Bearer) -> Option<JWToken> {
    let key: Hmac<Sha512> = Hmac::new_from_slice(SECRET_KEY.as_bytes()).unwrap();
    let token: Token<Header, BTreeMap<String, String>, _> =
        VerifyWithKey::verify_with_key(bearer.token(), &key).ok()?;
    let claims = token.claims();
    Some(JWToken {
        id: claims.get("id")?.into(),
        sub: claims.get("sub")?.parse().ok()?,
        iss: claims.get("iss")?.into(),
        iat: claims.get("iat")?.parse().ok()?,
        exp: claims.get("exp")?.parse().ok()?,
    })
}

pub fn generate_jwt(sub: bool, id: &str) -> String {
    let key: Hmac<Sha512> = Hmac::new_from_slice(SECRET_KEY.as_bytes()).unwrap();
    let header = Header {
        algorithm: jwt::AlgorithmType::Hs512,
        ..Default::default()
    };
    let mut claims = BTreeMap::new();
    // 签发者
    claims.insert("iss", "CRM-SHA-1".into());
    claims.insert("id", id.into());
    // 用户
    claims.insert("sub", sub.to_string());
    let time = TIME::now().expect("Time go ahead");
    // 设置token签发时间
    claims.insert("iat", time.naos().to_string());
    let local = chrono::Local.timestamp_nanos(time.naos() as i64);
    let next_week = local
        .checked_add_days(Days::new(7))
        .expect("token error-week");
    // 设置token过期时间
    claims.insert("exp", next_week.timestamp_nanos_opt().unwrap().to_string());
    Token::new(header, claims)
        .sign_with_key(&key)
        .unwrap()
        .as_str()
        .into()
}