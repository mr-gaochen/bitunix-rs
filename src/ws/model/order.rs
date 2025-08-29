use serde::{Deserialize, Serialize};

/// 订单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    /// 事件类型: CREATE / UPDATE / CLOSE
    pub event: String,

    /// 订单 ID
    #[serde(rename = "orderId")]
    pub order_id: String,

    /// 交易对
    pub symbol: String,

    /// 保证金模式: ISOLATION / CROSS
    #[serde(rename = "positionType")]
    pub position_type: String,

    /// 持仓模式: ONE_WAY / HEDGE
    #[serde(rename = "positionMode")]
    pub position_mode: String,

    /// 买卖方向: BUY / SELL
    pub side: String,

    /// 委托有效方式:
    /// - IOC: Immediate or Cancel
    /// - FOK: Fill or Kill
    /// - GTC: Good Till Canceled (默认)
    /// - POST_ONLY: Post Only
    pub effect: Option<String>,

    /// 订单类型: LIMIT / MARKET
    #[serde(rename = "type")]
    pub order_type: String,

    /// 下单数量（基础币种）
    pub qty: String,

    /// 下单价格（仅 LIMIT 必填）
    pub price: Option<String>,

    /// 创建时间
    pub ctime: String,

    /// 更新时间
    pub mtime: String,

    /// 杠杆倍数
    pub leverage: String,

    /// 订单状态:
    /// INIT / NEW / PART_FILLED / CANCELED / FILLED / PART_FILLED_CANCELED
    #[serde(rename = "orderStatus")]
    pub order_status: String,

    /// 手续费
    pub fee: Option<String>,

    /// 平均成交价
    #[serde(rename = "averagePrice")]
    pub average_price: Option<String>,

    /// 成交数量
    #[serde(rename = "dealAmount")]
    pub deal_amount: Option<String>,

    /// 客户端订单 ID
    #[serde(rename = "clientId")]
    pub client_id: Option<String>,

    /// 止盈触发类型: MARK_PRICE / LAST_PRICE
    #[serde(rename = "tpStopType")]
    pub tp_stop_type: Option<String>,

    /// 止盈触发价格
    #[serde(rename = "tpPrice")]
    pub tp_price: Option<String>,

    /// 止盈委托类型: LIMIT / MARKET
    #[serde(rename = "tpOrderType")]
    pub tp_order_type: Option<String>,

    /// 止盈下单价格
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

    /// 止损下单价格
    #[serde(rename = "slOrderPrice")]
    pub sl_order_price: Option<String>,
}
