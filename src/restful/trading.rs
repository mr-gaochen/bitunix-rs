use crate::client::BitUnixClient;

use super::models::{RestApi, TradePlceOrder};
use anyhow::Result;
use serde_json::Value;
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
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("symbol".into(), symbol.into());
        params.insert("qty".into(), qty.into());
        params.insert("side".into(), side.into());
        params.insert("orderType".into(), order_type.into());
        if let Some(trade_side) = trade_side {
            params.insert("tradeSide".into(), trade_side.into());
        }
        if let Some(price) = price {
            params.insert("price".into(), price.into());
        }
        if let Some(postion_id) = postion_id {
            params.insert("positionId".into(), postion_id.into());
        }
        if let Some(effect) = effect {
            params.insert("effect".into(), effect.into());
        }
        if let Some(client_id) = client_id {
            params.insert("clientId".into(), client_id.into());
        }

        // Ok(self
        //     .post::<RestApi<Value>>("/api/v1/futures/trade/place_order", &params)
        //     .await?)

        Ok(self
            .post::<RestApi<TradePlceOrder>>("/api/v1/futures/trade/place_order", &params)
            .await?)
    }
}
