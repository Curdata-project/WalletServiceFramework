use serde::Deserialize;
use serde::{Deserializer, Serializer};
use serde_json::Value;

pub fn deserialize_value<'de, D>(deserializer: D) -> Result<Value, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    Ok(serde_json::from_str(&s).unwrap())
}

pub fn serialize_value<SE>(obj: &Value, serializer: SE) -> Result<SE::Ok, SE::Error>
where
    SE: Serializer,
{
    serializer.serialize_str(&obj.to_string())
}
