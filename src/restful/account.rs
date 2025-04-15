use super::models::{AccountData, RestApi};
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
}
