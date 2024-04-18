use std::{
    collections::HashMap,
    sync::Arc,
};

use crate::{database::get_conn, pages::account::User};
use mysql::prelude::Queryable;
use tokio::sync::RwLock;
lazy_static::lazy_static! {
    pub static ref USER_CACHE: Arc<RwLock<HashMap<String, User>>> = {
        let mut conn = get_conn().expect("连接数据库失败");
        let users: Vec<User> = conn.query(
            "select u.* from user u where not exists (select 1 from leaver l where l.id=u.id)").expect("加载用户数据失败");
        Arc::new(RwLock::new(users.into_iter().map(|u| (u.id.clone(), u)).collect()))
    };
}