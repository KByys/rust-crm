use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;

use crate::pages::User;
// pub fn clear_cache() {
//     ORDER_CACHE.clear();
//     CUSTOMER_CACHE.clear();
//     PRODUCT_CACHE.clear();
//     OPTION_CACHE.clear();
//     USER_CACHE.clear();
// }
// pub type Cache = Arc<DashMap<String, DashMap<String, Value>>>;
// lazy_static::lazy_static! {
//     pub static ref ORDER_CACHE: Cache = {
//         Arc::new(DashMap::new())
//     };
//     pub static ref CUSTOMER_CACHE: Cache = {
//         Arc::new(DashMap::new())
//     };
//     pub static ref PRODUCT_CACHE: Arc<DashMap<String, Value>> = {
//         Arc::new(DashMap::new())
//     };
//     pub static ref OPTION_CACHE: Cache = {
//         Arc::new(DashMap::new())
//     };
//     pub static ref USER_CACHE: Arc<DashMap<String, User>> = {
//         Arc::new(DashMap::new())
//     };
//     pub static ref TOKEN_CACHE: Arc<DashMap<String, Vec<String>>> = {
//         Arc::new(DashMap::new())
//     };
// }

macro_rules! gen_cache {
    ($(($N:ident, $T:ty)), +) => {
        lazy_static::lazy_static! {
            $(
                pub static ref $N: Arc<DashMap<String, $T>> = {
                    Arc::new(DashMap::new())
                };
            )+
        }
        pub fn clear_cache() {
            $(
                $N.clear();
            )+
        }
    };
}

gen_cache!{
    (ORDER_CACHE, DashMap<String, Value>),
    (CUSTOMER_CACHE, DashMap<String, Value>),
    (PRODUCT_CACHE, Value),
    (OPTION_CACHE, DashMap<String, Value>),
    (USER_CACHE, User),
    (TOKEN_CACHE, String)
}
