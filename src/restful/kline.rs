use crate::client::BitUnixClient;
use anyhow::Result;
use std::collections::BTreeMap;

use super::models::{KLineData, RestApi};

impl BitUnixClient {
    /// 获取K线数据
    /// curl -X 'GET'  --location 'https://fapi.bitunix.com/api/v1/futures/market/kline?symbol=BTCUSDT&startTime=1&endTime=10234&interval=15m'
    pub async fn futures_market_kline(
        &self,
        symbol: &str,
        start_time: i64,
        end_time: i64,
        interval: &str,
    ) -> Result<RestApi<KLineData>> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("symbol".into(), symbol.into());
        params.insert("startTime".into(), start_time.to_string());
        params.insert("endTime".into(), end_time.to_string());
        params.insert("interval".into(), interval.into());
        Ok(self
            .get::<RestApi<KLineData>>("/api/v1/futures/market/kline", &params)
            .await?)
    }
}
