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

/// 获取历史持仓信息
///GET /api/v1/futures/position/get_history_positions
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPositionModel {
    #[serde(rename = "positionId")]
    pub position_id: String,
    pub symbol: String,
    #[serde(rename = "maxQty")]
    pub max_qty: String,
    #[serde(rename = "entryPrice")]
    pub entry_price: String,
    #[serde(rename = "closePrice")]
    pub close_price: String,
    #[serde(rename = "liqQty")]
    pub liq_qty: String,
    pub side: String, // 可选: 定义 enum PositionSide { LONG, SHORT }
    #[serde(rename = "marginMode")]
    pub margin_mode: String, // 可选: enum MarginMode { ISOLATION, CROSS }
    #[serde(rename = "positionMode")]
    pub position_mode: String, // 可选: enum PositionMode { ONE_WAY, HEDGE }
    pub leverage: i32,
    pub fee: String,
    pub funding: String,
    #[serde(rename = "realizedPNL")]
    pub realized_pnl: String,
    #[serde(rename = "liqPrice")]
    pub liq_price: String,
    pub ctime: i64,
    pub mtime: i64,
}
///GET /api/v1/futures/position/get_history_positions
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPostions {
    pub total: i64,
    #[serde(rename = "positionList")]
    pub position_list: Vec<HistoryPositionModel>,
}
