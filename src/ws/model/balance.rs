use serde::{Deserialize, Serialize};

/// 账户余额信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    /// 币种
    pub coin: String,
    /// 可用余额
    pub available: String,
    /// 总冻结余额 = isolationFrozen + crossFrozen
    pub frozen: String,
    /// 仓位内冻结资金
    pub isolationFrozen: String,
    /// 全仓冻结资金
    pub crossFrozen: String,
    /// 保证金（总额）
    pub margin: String,
    /// 分仓保证金
    pub isolationMargin: String,
    /// 全仓保证金
    pub crossMargin: String,
    /// 体验金
    pub expMoney: String,
}
