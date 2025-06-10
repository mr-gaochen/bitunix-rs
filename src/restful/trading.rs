use crate::client::BitUnixClient;

use super::models::{CancleOrders, OrderData, RestApi, TradePlceOrder};
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::BTreeMap;

impl BitUnixClient {
    ///  下单
    ///POST /api/v1/futures/trade/place_order
    pub async fn trade_place_order(
        &self,
        symbol: &str,
        qty: &str,
        side: &str,
        trade_side: &str,
        order_type: &str,
        price: Option<&str>,
        postion_id: Option<&str>,
        effect: Option<&str>,
        client_id: Option<&str>,
        tp_price: Option<&str>,
        tp_stop_type: Option<&str>,
        tp_order_type: Option<&str>,
        tp_order_price: Option<&str>,
        sl_price: Option<&str>,
        sl_stop_type: Option<&str>,
        sl_order_type: Option<&str>,
        sl_order_price: Option<&str>,
    ) -> Result<RestApi<TradePlceOrder>> {
        let mut params: BTreeMap<String, Value> = BTreeMap::new();
        params.insert("symbol".into(), json!(symbol));
        params.insert("qty".into(), json!(qty));
        params.insert("side".into(), json!(side));
        params.insert("orderType".into(), json!(order_type));
        params.insert("tradeSide".into(), json!(trade_side));
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
        if let Some(tp_price) = tp_price {
            params.insert("tpPrice".into(), json!(tp_price));
        }
        if let Some(tp_stop_type) = tp_stop_type {
            params.insert("tpStopType".into(), json!(tp_stop_type));
        }
        if let Some(tp_order_type) = tp_order_type {
            params.insert("tpOrderType".into(), json!(tp_order_type));
        }
        if let Some(tp_order_price) = tp_order_price {
            params.insert("tpOrderPrice".into(), json!(tp_order_price));
        }

        if let Some(sl_price) = sl_price {
            params.insert("slPrice".into(), json!(sl_price));
        }
        if let Some(sl_stop_type) = sl_stop_type {
            params.insert("slStopType".into(), json!(sl_stop_type));
        }
        if let Some(sl_order_type) = sl_order_type {
            params.insert("slOrderType".into(), json!(sl_order_type));
        }
        if let Some(sl_order_price) = sl_order_price {
            params.insert("slOrderPrice".into(), json!(sl_order_price));
        }

        Ok(self
            .post::<RestApi<TradePlceOrder>>("/api/v1/futures/trade/place_order", &params)
            .await?)
    }

    /// 取消订单
    /// POST /api/v1/futures/trade/cancel_orders
    pub async fn cancle_orders(
        &self,
        symbol: &str,
        order_list: Vec<TradePlceOrder>,
    ) -> Result<RestApi<CancleOrders>> {
        let mut params: BTreeMap<String, Value> = BTreeMap::new();
        params.insert("symbol".into(), json!(symbol));
        params.insert("orderList".into(), json!(order_list));
        Ok(self
            .post::<RestApi<CancleOrders>>("/api/v1/futures/trade/cancel_orders", &params)
            .await?)
    }

    /// 查询订单详情
    /// GET  /api/v1/futures/trade/get_order_detail
    pub async fn order_details(&self, order_id: &str) -> Result<RestApi<OrderData>> {
        let mut params: BTreeMap<String, String> = BTreeMap::new();
        params.insert("orderId".into(), order_id.into());
        Ok(self
            .get::<RestApi<OrderData>>("/api/v1/futures/trade/get_order_detail", &params)
            .await?)
    }

    /// 平仓所有仓位
    pub async fn close_all_position(&self, symbol: &str) -> Result<RestApi<Value>> {
        let mut params: BTreeMap<String, Value> = BTreeMap::new();
        params.insert("symbol".into(), symbol.into());
        Ok(self
            .post::<RestApi<Value>>("/api/v1/futures/trade/close_all_position", &params)
            .await?)
    }
}
