use serde::{Deserialize, Serialize};

/// 止盈止损订单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TpSlOrder {
    /// 事件类型: CREATE / UPDATE / CLOSE
    pub event: String,

    /// 仓位 ID
    #[serde(rename = "positionId")]
    pub position_id: String,

    /// 订单 ID
    #[serde(rename = "orderId")]
    pub order_id: String,

    /// 交易对
    pub symbol: String,

    /// 杠杆倍数
    pub leverage: String,

    /// 买卖方向: BUY / SELL
    pub side: String,

    /// 持仓模式: ONE_WAY / HEDGE
    #[serde(rename = "positionMode")]
    pub position_mode: String,

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
    #[serde(rename = "tpQty")]
    pub tp_qty: Option<String>,

    /// 止损委托数量（基础币种）
    #[serde(rename = "slQty")]
    pub sl_qty: Option<bool>,

    /// 止盈触发类型: MARK_PRICE / LAST_PRICE
    #[serde(rename = "tpStopType")]
    pub tp_stop_type: Option<String>,

    /// 止盈触发价格
    #[serde(rename = "tpPrice")]
    pub tp_price: Option<String>,

    /// 止盈委托类型: LIMIT / MARKET
    #[serde(rename = "tpOrderType")]
    pub tp_order_type: Option<String>,

    /// 止盈委托下单价格
    #[serde(rename = "tpOrderPrice")]
    pub tp_order_price: Option<String>,

    /// 止损触发类型: MARK_PRICE / LAST_PRICE
    #[serde(rename = "slStopType")]
    pub sl_stop_type: Option<String>,

    /// 止损触发价格
    #[serde(rename = "slPrice")]
    pub sl_price: Option<String>,

    /// 止损委托类型: LIMIT / MARKET
    #[serde(rename = "slOrderType")]
    pub sl_order_type: Option<String>,

    /// 止损委托下单价格
    #[serde(rename = "slOrderPrice")]
    pub sl_order_price: Option<String>,
}
