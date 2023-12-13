use axum::{http::HeaderMap, Json};
use mysql::prelude::Queryable;
use serde_json::{json, Value};

use crate::{
    bearer,
    database::get_conn,
    debug_info,
    response::Response,
    token::{generate_jwt, parse_jwt, TokenVerification},
    ResponseResult,
};

use super::User;

#[derive(serde::Deserialize)]
struct LoginID {
    id: String,
    password: String,
}

pub async fn user_login(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    if let Some(bearer) = bearer!(&headers, Allow Missing) {
        let token = match parse_jwt(&bearer) {
            Some(token) if !token.sub => {
                return Err(Response::token_error("客户账号无法进行员工登录"))
            }
            None => return Err(Response::token_error("Invalid token")),
            Some(token) => token,
        };
        let btn: Option<i64> = conn.query_first(format!(
            "SELECT btn FROM token WHERE ty = 0 AND id = '{}'",
            token.id
        ))?;
        if btn.is_some_and(|btn| btn >= token.iat) {
            return Err(Response::token_error("token已过期，无法刷新，请重新登录"));
        }

        match token.verify(&mut conn)? {
            TokenVerification::Ok => {
                let data: Option<User> =
                    conn.query_first(format!("SELECT * FROM user WHERE id = {}", token.id))?;
                Ok(Response::ok(json!({
                    "token": bearer.token(),
                    "info": data
                })))
            }
            TokenVerification::Expired => {
                if token.is_refresh() {
                    let data: Option<User> =
                        conn.query_first(format!("SELECT * FROM user WHERE id = '{}'", token.id))?;
                    let token = generate_jwt(true, &token.id);
                    Ok(Response::ok(json!({
                        "token": token,
                        "info": data
                    })))
                } else {
                    Err(Response::token_error("Token已过期"))
                }
            }
            TokenVerification::Error => Err(Response::token_error("Invalid token")),
        }
    } else {
        let user: LoginID = serde_json::from_value(value)?;
        let digest = md5::compute(&user.password);
        println!("123 -- {:?}", digest.0);
        let info: Option<User> =
            conn.query_first(format!("SELECT * FROM user WHERE id = '{}'", user.id))?;
        println!("{:?}", info);
        if let Some(user) = info {
            if user.password.as_slice() != digest.0.as_slice() {
                Err(Response::wrong_password())
            } else {
                let token = generate_jwt(true, &user.id);
                Ok(Response::ok(json!({"token": token, "info": user})))
            }
        } else {
            Err(Response::not_exist(format!("{} 用户不存在", user.id)))
        }
    }
}

pub async fn customer_login(headers: HeaderMap, Json(value): Json<Value>) -> ResponseResult {
    let mut conn = get_conn()?;
    if let Some(bearer) = bearer!(&headers, Allow Missing) {
        let token = match parse_jwt(&bearer) {
            Some(token) if token.sub => {
                return Err(Response::token_error("员工账号无法进行客户登录"))
            }
            None => return Err(Response::token_error("Invalid token")),
            Some(token) => token,
        };
        match token.verify(&mut conn)? {
            TokenVerification::Ok => {
                debug_info(format!("客户 {} 登录成功", token.id));
                Ok(Response::ok(json!({
                    "token": bearer.token(),
                    "id": token.id
                })))
            }
            TokenVerification::Expired => {
                if token.is_refresh() {
                    Ok(Response::ok(json!({
                        "token": generate_jwt(false, &token.id),
                        "id": token.id
                    })))
                } else {
                    Err(Response::token_error("Invalid token"))
                }
            }
            TokenVerification::Error => Err(Response::token_error("Invalid token")),
        }
    } else {
        let user: LoginID = serde_json::from_value(value)?;
        let digest = md5::compute(&user.password);
        let password: Option<Vec<u8>> = conn.query_first(format!(
            "SELECT password FROM customer_login WHERE id = '{}'",
            user.id
        ))?;
        if let Some(password) = password {
            if password.as_slice() == digest.0.as_slice() {
                Err(Response::wrong_password())
            } else {
                let token = generate_jwt(false, &user.id);
                Ok(Response::ok(json!({"token": token, "id": user.id})))
            }
        } else {
            Err(Response::not_exist(format!("{} 用户不存在", user.id)))
        }
    }
}
