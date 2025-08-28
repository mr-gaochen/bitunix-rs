use serde::{Deserialize, Serialize};

/// 仓位信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// 事件类型: OPEN / UPDATE / CLOSE
    pub event: String,
    /// 仓位 ID
    pub positionId: String,
    /// 保证金模式: ISOLATION / CROSS
    pub marginMode: String,
    /// 持仓模式: ONE_WAY / HEDGE
    pub positionMode: String,
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
    pub realizedPNL: String,
    /// 未实现盈亏
    pub unrealizedPNL: String,
    /// 总资金费用
    pub funding: String,
    /// 总手续费
    pub fee: String,
}
