use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;

use crate::{do_if, perm::roles::ROLE_TABLES};

pub fn deserialize_bool_to_i32<'de, D>(de: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: bool = Deserialize::deserialize(de)?;
    Ok(do_if!(value => 0, 1))
}
pub fn serialize_i32_to_bool<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
where
    T: Serialize + Display,
    S: Serializer,
{
    let flag = value.to_string().parse().unwrap_or(1);
    if flag == 0 {
        serializer.serialize_bool(true)
    } else {
        serializer.serialize_bool(false)
    }
}

pub fn serialize_null_to_default<S>(
    value: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(value) => serializer.serialize_str(value),
        _ => serializer.serialize_str(""),
    }
}
#[macro_export]
macro_rules! async_block {
    ($asy:expr) => {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on($asy)
    };
}



pub fn serialize_role<S>(id: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let name = unsafe {
        ROLE_TABLES.get_name(id).map_or(id.into(), |v|v.to_string())
    };

    serializer.serialize_str(&name)
}

pub fn deserialize_role<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let name: String = Deserialize::deserialize(de)?;
    let id = unsafe {
        ROLE_TABLES.get_id(&name).map_or(name, |v|v.to_string())
    };
    Ok(id)
}