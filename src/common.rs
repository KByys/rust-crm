use mysql::prelude::FromValue;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Person {
    pub name: String,
    pub phone: String,
}

impl Person {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn phone(&self) -> &str {
        &self.phone
    }
}
impl From<String> for Person {
    fn from(value: String) -> Self {
        Self {
            name: String::new(),
            phone: value,
        }
    }
}
impl FromValue for Person {
    type Intermediate = String;
}
impl<'de> serde::Deserialize<'de> for Person {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let phone: String = serde::Deserialize::deserialize(deserializer)?;
        Ok(Person {
            phone,
            name: String::new(),
        })
    }
}
pub fn empty_deserialize_to_none<'de, D, T: From<String>>(de: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<String> = Deserialize::deserialize(de)?;
    Ok(value.and_then(|v| op::ternary!(v.is_empty() => None; Some(T::from(v)))))
}
