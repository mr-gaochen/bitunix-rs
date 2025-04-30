use serde::de;
use serde::{Deserialize, Deserializer};

pub fn de_float_from_str<'a, D>(deserializer: D) -> Result<f64, D::Error>
where
    D: Deserializer<'a>,
{
    let str_val = String::deserialize(deserializer)?;
    if str_val == "".to_string() {
        return Ok(0.0);
    }
    str_val.parse::<f64>().map_err(de::Error::custom)
}

pub fn de_i64_from_str<'a, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: Deserializer<'a>,
{
    let str_val = String::deserialize(deserializer)?;
    if str_val.trim().is_empty() {
        return Ok(0);
    }
    str_val.parse::<i64>().map_err(de::Error::custom)
}
