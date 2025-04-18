use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct RestApi<T> {
    pub code: i32,
    pub msg: String,
    pub data: T,
}

/// 获取账户信息
/// GET /api/v1/futures/account
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountData {
    pub margin_coin: String, // 保证金币
    pub available: String,   // 可用数量
    pub frozen: String,      // 锁定订单数量
    pub margin: String,      // 锁定的仓位数量
    pub transfer: String,
    pub position_mode: String, // 持仓模式
    #[serde(rename = "isolationUnrealizedPNL")]
    pub isolation_unrealized_pnl: String,
    #[serde(rename = "crossUnrealizedPNL")]
    pub cross_unrealized_pnl: String,
    pub bonus: String,
}

/// 下单
///POST /api/v1/futures/trade/place_order
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradePlceOrder {
    pub order_id: String,
    pub client_id: String,
}
