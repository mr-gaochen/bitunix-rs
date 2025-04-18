use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct RestApi<T> {
    pub code: i32,
    pub msg: String,
    pub data: Vec<T>,
}

/// GET /api/v1/futures/account
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountData {
    pub margin_coin: String,
    pub available: String,
    pub frozen: String,
    pub margin: String,
    pub transfer: String,
    pub position_mode: String,
    pub cross_unrealized_pnl: String,
    pub isolation_unrealized_pnl: String,
    pub bonus: String,
}
