use crate::client::BitUnixClient;

use super::models::{RestApi, TradePlceOrder};
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

impl BitUnixClient {
    ///  下单
    ///POST /api/v1/futures/trade/place_order
    pub async fn trade_place_order<T>(
        &self,
        symbol: T,
        qty: T,
        side: T,
        trade_side: Option<T>,
        order_type: T,
        price: Option<T>,
        postion_id: Option<T>,
        effect: Option<T>,
        client_id: Option<T>,
    ) -> Result<RestApi<TradePlceOrder>>
    where
        T: Into<String>,
    {
        let mut params: BTreeMap<String, Value> = BTreeMap::new();
        params.insert("symbol".into(), json!(symbol.into()));
        params.insert("qty".into(), json!(qty.into()));
        params.insert("side".into(), json!(side.into()));
        params.insert("orderType".into(), json!(order_type.into()));
        if let Some(trade_side) = trade_side {
            params.insert("tradeSide".into(), json!(trade_side.into()));
        }
        if let Some(price) = price {
            params.insert("price".into(), json!(price.into()));
        }
        if let Some(postion_id) = postion_id {
            params.insert("positionId".into(), json!(postion_id.into()));
        }
        if let Some(effect) = effect {
            params.insert("effect".into(), json!(effect.into()));
        }
        if let Some(client_id) = client_id {
            params.insert("clientId".into(), json!(client_id.into()));
        }
        params.insert("reduceOnly".into(), json!(false));

        Ok(self
            .post::<RestApi<TradePlceOrder>>("/api/v1/futures/trade/place_order", &params)
            .await?)
    }
}
