use serde::{Deserialize, Serialize};

/// 仓位信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
    /// 事件类型: OPEN / UPDATE / CLOSE
    pub event: String,
    /// 仓位 ID
    #[serde(rename = "positionId")]
    pub position_id: String,
    /// 保证金模式: ISOLATION / CROSS
    #[serde(rename = "marginMode")]
    pub margin_mode: String,
    /// 持仓模式: ONE_WAY / HEDGE
    #[serde(rename = "positionMode")]
    pub position_mode: String,
    /// 持仓方向: SHORT / LONG
    pub side: String,
    /// 杠杆倍数
    pub leverage: String,
    /// 保证金
    pub margin: String,
    /// 创建时间 (ISO 8601 格式)
    pub ctime: String,
    /// 开仓数量
    pub qty: String,
    /// 交易对
    pub symbol: String,
    /// 已实现盈亏（不含资金费与交易费）
    #[serde(rename = "realizedPNL")]
    pub realized_pnl: String,
    /// 未实现盈亏
    #[serde(rename = "unrealizedPNL")]
    pub unrealized_pnl: String,
    /// 总资金费用
    pub funding: String,
    /// 总手续费
    pub fee: String,
}
