use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::Display;

use crate::perm::roles::ROLE_TABLES;

pub fn deser_f32<'de, D>(de: D) -> Result<f32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: String = Deserialize::deserialize(de)?;
    if let Ok(f) = value.parse::<f32>() {
        Ok(f)
    } else {
        Err(serde::de::Error::custom("price不是浮点数格式"))
    }
}


pub fn split_files<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error> 
where
    S: Serializer
{
     Serialize::serialize(&value.split('/').collect::<Vec<&str>>(), serializer)
}

pub fn serialize_f32_to_string<S>(value: &f32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(value.to_string().as_str())
}

pub fn deserialize_bool_to_i32<'de, D>(de: D) -> Result<i32, D::Error>
where
    D: Deserializer<'de>,
{
    let value: bool = Deserialize::deserialize(de)?;
    Ok(op::ternary!(value => 0; 1))
}
pub fn serialize_empty_to_none<S>(value: &Option<String>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match value {
        Some(k) if !k.is_empty() => serializer.serialize_str(k),
        _ => serializer.serialize_none(),
    }
}
pub fn deser_empty_to_none<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(de)?;
    Ok(value.and_then(|s| op::ternary!(s.is_empty() => None; Some(s))))
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

// pub fn serialize_custom_fields(
//     value: &HashMap<String, Vec<crate::pages::>>,
//     serializer: S,
// ) -> Result<S::Ok, S::Error>
// where
//     S: Serializer,
// {
//     match value {
//         Some(value) => serializer.serialize_str(value),
//         _ => serializer.serialize_str(""),
//     }
// }

pub fn serialize_role<S>(id: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let name = unsafe {
        ROLE_TABLES
            .get_name(id)
            .map_or(id.into(), |v| v.to_string())
    };

    serializer.serialize_str(&name)
}

pub fn deserialize_role<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let name: String = Deserialize::deserialize(de)?;
    let id = unsafe { ROLE_TABLES.get_id(&name).map_or(name, |v| v.to_string()) };
    Ok(id)
}

pub fn deserialize_roles<'de, D>(de: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let roles: Vec<String> = Deserialize::deserialize(de)?;
    Ok(unsafe {
        roles
            .into_iter()
            .map(|r| ROLE_TABLES.get_id(&r).map_or(r, |v| v.to_string()))
    }
    .collect())
}

pub fn deserialize_mm_dd<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(r"(\d{2})-(\d{2})", de, "MM-DD")
}
fn regex_time<'de, D: Deserializer<'de>>(re: &str, de: D, err: &str) -> Result<String, D::Error> {
    let time: String = Deserialize::deserialize(de)?;
    if time.is_empty() {
        return Ok(String::new());
    }
    let regex = Regex::new(re).unwrap();
    if regex.is_match(&time) {
        Ok(time)
    } else {
        Err(serde::de::Error::custom(format!(
            "Invalid Time Format. 时间格式应当为'{err}'"
        )))
    }
}

pub fn deser_yyyy_mm_dd_hh_mm_ss<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(
        r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})",
        de,
        "YYYY-MM-DD HH:MM:SS",
    )
}
pub fn deser_yyyy_mm_dd_hh_mm<'de, D>(de: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(
        r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})",
        de,
        "YYYY-MM-DD HH:MM",
    )
}

pub fn op_deser_yyyy_mm_dd_hh_mm<'de, D>(de: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    regex_time(
        r"(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2})",
        de,
        "YYYY-MM-DD HH:MM",
    )
    .map(|s| op::ternary!(s.is_empty() => None; Some(s)))
}
