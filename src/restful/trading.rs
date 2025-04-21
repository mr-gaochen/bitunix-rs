use crate::client::BitUnixClient;

use super::models::{RestApi, TradePlceOrder};
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

impl BitUnixClient {
    ///  下单
    ///POST /api/v1/futures/trade/place_order
    pub async fn trade_place_order(
        &self,
        symbol: String,
        qty: String,
        side: String,
        trade_side: Option<String>,
        order_type: String,
        price: Option<String>,
        postion_id: Option<String>,
        effect: Option<String>,
        client_id: Option<String>,
    ) -> Result<RestApi<TradePlceOrder>> {
        let mut params: BTreeMap<String, Value> = BTreeMap::new();
        params.insert("symbol".into(), json!(symbol));
        params.insert("qty".into(), json!(qty));
        params.insert("side".into(), json!(side));
        params.insert("orderType".into(), json!(order_type));
        if let Some(trade_side) = trade_side {
            params.insert("tradeSide".into(), json!(trade_side));
        }
        if let Some(price) = price {
            params.insert("price".into(), json!(price));
        }
        if let Some(postion_id) = postion_id {
            params.insert("positionId".into(), json!(postion_id));
        }
        if let Some(effect) = effect {
            params.insert("effect".into(), json!(effect));
        }
        if let Some(client_id) = client_id {
            params.insert("clientId".into(), json!(client_id));
        }
        params.insert("reduceOnly".into(), json!(false));

        Ok(self
            .post::<RestApi<TradePlceOrder>>("/api/v1/futures/trade/place_order", &params)
            .await?)
    }
}
