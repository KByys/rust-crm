use std::fmt::Display;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::do_if;

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
