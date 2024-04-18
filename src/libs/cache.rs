use std::sync::Arc;

use dashmap::DashMap;
use serde_json::Value;

pub type Cache = Arc<DashMap<String, DashMap<String, Value>>>;
lazy_static::lazy_static!{
    pub static ref ORDER_CACHE: Cache = {
        Arc::new(DashMap::new())
    };
    pub static ref CUSTOMER_CACHE: Cache = {
        Arc::new(DashMap::new())
    };
    pub static ref PRODUCT_CACHE: Arc<DashMap<String, Value>> = {
        Arc::new(DashMap::new())
    };
    pub static ref OPTION_CACHE: Cache = {
        Arc::new(DashMap::new())
    };
}