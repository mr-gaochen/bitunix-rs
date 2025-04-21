use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug)]
pub struct RestApi<T> {
    pub code: i32,
    pub msg: String,
    pub data: Option<T>,
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
    #[serde(rename = "orderId")]
    pub order_id: String,
    #[serde(rename = "clientId")]
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
    pub leverage: String,
    pub fee: String,
    pub funding: String,
    #[serde(rename = "realizedPNL")]
    pub realized_pnl: String,
    #[serde(rename = "liqPrice")]
    pub liq_price: String,
    pub ctime: String,
    pub mtime: String,
}
///GET /api/v1/futures/position/get_history_positions
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryPostions {
    pub total: String,
    #[serde(rename = "positionList")]
    pub position_list: Vec<HistoryPositionModel>,
}

///Get History Trades
///GET /api/v1/futures/trade/get_history_trades
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeListData {
    #[serde(rename = "tradeList")]
    pub trade_list: Vec<TradeInfo>,
    pub total: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeInfo {
    #[serde(rename = "tradeId")]
    pub trade_id: String,
    #[serde(rename = "orderId")]
    pub order_id: String,
    pub qty: String,
    pub price: String,
    #[serde(rename = "symbol")]
    pub symbol: String,
    #[serde(rename = "positionMode")]
    pub position_mode: String,
    #[serde(rename = "marginMode")]
    pub margin_mode: String,
    pub leverage: i64,
    pub fee: String,
    #[serde(rename = "realizedPNL")]
    pub realized_pnl: String,
    #[serde(rename = "orderType")]
    pub order_type: String,
    #[serde(rename = "reduceOnly")]
    pub reduce_only: bool,
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,
    #[serde(rename = "source")]
    pub source: Option<String>,
    #[serde(rename = "ctime")]
    pub ctime: String,
    #[serde(rename = "effect")]
    pub effect: Option<String>,
    #[serde(rename = "marginCoin")]
    pub margin_coin: Option<String>,
    #[serde(rename = "roleType")]
    pub role_type: String,
    pub side: String,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionData {
    pub position_id: String,
    pub symbol: String,
    pub qty: String,
    pub entry_value: String,
    pub side: String,
    pub position_mode: String,
    pub margin_mode: String,
    pub leverage: String,
    pub fee: String,
    pub funding: String,
    pub realized_pnl: String,
    pub margin: String,
    pub unrealized_pnl: String,
    pub liq_price: String,
    pub margin_rate: String,
    pub avg_open_price: String,
    pub ctime: i64,
    pub mtime: i64,
}
