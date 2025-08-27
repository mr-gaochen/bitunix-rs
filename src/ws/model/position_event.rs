use serde::Deserialize;

/// 仓位推送结构体
#[derive(Debug, Deserialize, Clone)]
pub struct PositionEvent {
    pub event: String,
    #[serde(rename = "positionId")]
    pub position_id: String,
    #[serde(rename = "marginMode")]
    pub margin_mode: String,
    #[serde(rename = "positionMode")]
    pub position_mode: String,
    pub side: String,
    pub leverage: String,
    pub margin: String,
    #[serde(rename = "ctime")]
    pub create_time: String,
    pub qty: String,
    pub symbol: String,
    #[serde(rename = "realizedPNL")]
    pub realized_pnl: String,
    #[serde(rename = "unrealizedPNL")]
    pub unrealized_pnl: String,
    pub funding: String,
    pub fee: String,
}
