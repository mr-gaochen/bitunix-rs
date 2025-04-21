use super::models::{AccountData, HistoryPostions, RestApi};
use crate::client::BitUnixClient;
use anyhow::Result;
use std::collections::BTreeMap;

impl BitUnixClient {
    // 查看账户信息
    // GET /api/v1/futures/account
    pub async fn account(&self, margin_coin: String) -> Result<RestApi<AccountData>> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("marginCoin".into(), margin_coin.into());
        Ok(self
            .get::<RestApi<AccountData>>("/api/v1/futures/account", &params)
            .await?)
    }

    // 获取历史持仓
    // /api/v1/futures/position/get_history_positions
    pub async fn histroy_postions(
        &self,
        symbol: Option<String>,
    ) -> Result<RestApi<HistoryPostions>> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        if let Some(symbol) = symbol {
            params.insert("symbol".into(), symbol.into());
        }
        Ok(self
            .get::<RestApi<HistoryPostions>>(
                "/api/v1/futures/position/get_history_positions",
                &params,
            )
            .await?)
    }
}
