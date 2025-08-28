use serde::{Deserialize, Serialize};

/// 订单信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// 事件类型: CREATE / UPDATE / CLOSE
    pub event: String,

    /// 订单 ID
    pub orderId: String,

    /// 交易对
    pub symbol: String,

    /// 保证金模式: ISOLATION / CROSS
    pub positionType: String,

    /// 持仓模式: ONE_WAY / HEDGE
    pub positionMode: String,

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
    pub orderStatus: String,

    /// 手续费
    pub fee: Option<String>,

    /// 平均成交价
    pub averagePrice: Option<String>,

    /// 成交数量
    pub dealAmount: Option<String>,

    /// 客户端订单 ID
    pub clientId: Option<String>,

    /// 止盈触发类型: MARK_PRICE / LAST_PRICE
    pub tpStopType: Option<String>,

    /// 止盈触发价格
    pub tpPrice: Option<String>,

    /// 止盈委托类型: LIMIT / MARKET
    pub tpOrderType: Option<String>,

    /// 止盈下单价格
    pub tpOrderPrice: Option<String>,

    /// 止损触发类型: MARK_PRICE / LAST_PRICE
    pub slStopType: Option<String>,

    /// 止损触发价格
    pub slPrice: Option<String>,

    /// 止损委托类型: LIMIT / MARKET
    pub slOrderType: Option<String>,

    /// 止损下单价格
    pub slOrderPrice: Option<String>,
}
