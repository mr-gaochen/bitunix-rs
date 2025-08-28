use serde::{Deserialize, Serialize};

/// 止盈止损订单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TpSlOrder {
    /// 事件类型: CREATE / UPDATE / CLOSE
    pub event: String,

    /// 仓位 ID
    pub positionId: String,

    /// 订单 ID
    pub orderId: String,

    /// 交易对
    pub symbol: String,

    /// 杠杆倍数
    pub leverage: String,

    /// 买卖方向: BUY / SELL
    pub side: String,

    /// 持仓模式: ONE_WAY / HEDGE
    pub positionMode: String,

    /// 订单状态:
    /// INIT: 准备中
    /// NEW: 待执行
    /// PART_FILLED: 部分成交
    /// CANCELED: 已取消
    /// FILLED: 全部成交
    pub status: String,

    /// 创建时间戳 (ISO 8601 格式)
    pub ctime: String,

    /// 订单类型: LIMIT / MARKET
    #[serde(rename = "type")]
    pub order_type: String,

    /// 止盈委托数量（基础币种）
    pub tpQty: Option<String>,

    /// 止损委托数量（基础币种）
    pub slQty: Option<bool>,

    /// 止盈触发类型: MARK_PRICE / LAST_PRICE
    pub tpStopType: Option<String>,

    /// 止盈触发价格
    pub tpPrice: Option<String>,

    /// 止盈委托类型: LIMIT / MARKET
    pub tpOrderType: Option<String>,

    /// 止盈委托下单价格
    pub tpOrderPrice: Option<String>,

    /// 止损触发类型: MARK_PRICE / LAST_PRICE
    pub slStopType: Option<String>,

    /// 止损触发价格
    pub slPrice: Option<String>,

    /// 止损委托类型: LIMIT / MARKET
    pub slOrderType: Option<String>,

    /// 止损委托下单价格
    pub slOrderPrice: Option<String>,
}
