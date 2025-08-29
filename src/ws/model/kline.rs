use crate::utils::de_float_from_str;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KlineMessage {
    pub ch: String,
    pub symbol: String,
    pub ts: i64,
    pub data: KlineData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KlineData {
    #[serde(deserialize_with = "de_float_from_str")]
    pub o: f64, // 开盘价
    #[serde(deserialize_with = "de_float_from_str")]
    pub c: f64, // 收盘价
    #[serde(deserialize_with = "de_float_from_str")]
    pub h: f64, // 最高价
    #[serde(deserialize_with = "de_float_from_str")]
    pub l: f64, // 最低价
    #[serde(deserialize_with = "de_float_from_str")]
    pub b: f64, // 成交量 (base)
    #[serde(deserialize_with = "de_float_from_str")]
    pub q: f64, // 成交额 (quote)
}
