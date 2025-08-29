use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KlineMessage {
    pub ch: String,
    pub symbol: String,
    pub ts: i64,
    pub data: KlineData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KlineData {
    pub o: String, // 开盘价
    pub c: String, // 收盘价
    pub h: String, // 最高价
    pub l: String, // 最低价
    pub b: String, // 成交量 (base)
    pub q: String, // 成交额 (quote)
}