use serde::{Deserialize, Serialize};

/// 账户余额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
    /// 币种
    pub coin: String,
    /// 可用余额
    pub available: String,
    /// 总冻结余额 = isolationFrozen + crossFrozen
    pub frozen: String,
    /// 仓位内冻结资金
    #[serde(rename = "isolationFrozen")]
    pub isolation_frozen: String,
    /// 全仓冻结资金
    #[serde(rename = "crossFrozen")]
    pub cross_frozen: String,
    /// 保证金（总额）
    pub margin: String,
    /// 分仓保证金
    #[serde(rename = "isolationMargin")]
    pub isolation_margin: String,
    /// 全仓保证金
    #[serde(rename = "crossMargin")]
    pub cross_margin: String,
    /// 体验金
    #[serde(rename = "expMoney")]
    pub exp_money: String,
}
