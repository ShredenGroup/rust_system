use crate::common::consts::BINANCE_FUTURES_URL;
use crate::common::utils::generate_hmac_signature;
use crate::dto::binance::rest_api::{
    OrderType, OrderSide, TimeInForce, KlineRequest, KlineResponse,
    OrderRequest, OrderResponse
};
use anyhow::{Ok, Result};
use reqwest::Client;
use serde_json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 币安期货 API 客户端
#[derive(Debug, Clone)]
pub struct BinanceFuturesApi {
    base_url: String,
    client: Client,
    api_key: String,
    secret_key: String,
}

impl BinanceFuturesApi {
    /// 创建新的币安期货 API 客户端
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self {
            base_url: BINANCE_FUTURES_URL.to_string(),
            client: Client::new(),
            api_key,
            secret_key,
        }
    }

    /// 获取当前时间戳（毫秒）
    fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// 构建查询字符串
    fn build_query_string(&self, params: &HashMap<String, String>) -> String {
        let mut pairs: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        pairs.sort(); // 币安要求参数按字母顺序排序
        pairs.join("&")
    }

    /// 生成签名
    fn generate_signature(&self, query_string: &str) -> String {
        generate_hmac_signature(query_string, &self.secret_key)
    }

    /// 获取K线数据
    pub async fn get_klines(&self, request: &KlineRequest) -> Result<KlineResponse> {
        let params = request.to_params()?;
        let query = self.build_query_string(&params);
        let url = format!("{}/klines?{}", self.base_url, query);
        println!("Requesting URL: {}", url);
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API request failed: {}", error_text));
        }
        
        let klines = response.json().await?;
        Ok(klines)
    }

    /// 发送下单请求
    pub async fn new_order(&self, request: OrderRequest) -> Result<OrderResponse> {
        // 构建请求参数
        let mut params = HashMap::new();

        // 必需参数 - 使用 serde 序列化
        params.insert("symbol".to_string(), request.symbol.clone());
        params.insert(
            "side".to_string(),
            serde_json::to_string(&request.side)?
                .trim_matches('"')
                .to_string(),
        );
        params.insert(
            "type".to_string(),
            serde_json::to_string(&request.order_type)?
                .trim_matches('"')
                .to_string(),
        );

        // 可选参数
        if let Some(ref position_side) = request.position_side {
            params.insert("positionSide".to_string(), position_side.clone());
        }
        if let Some(ref time_in_force) = request.time_in_force {
            params.insert(
                "timeInForce".to_string(),
                serde_json::to_string(time_in_force)?
                    .trim_matches('"')
                    .to_string(),
            );
        }
        if let Some(ref quantity) = request.quantity {
            params.insert("quantity".to_string(), quantity.clone());
        }
        if let Some(ref reduce_only) = request.reduce_only {
            params.insert("reduceOnly".to_string(), reduce_only.clone());
        }
        if let Some(ref price) = request.price {
            params.insert("price".to_string(), price.clone());
        }
        if let Some(ref new_client_order_id) = request.new_client_order_id {
            params.insert("newClientOrderId".to_string(), new_client_order_id.clone());
        }
        if let Some(ref stop_price) = request.stop_price {
            params.insert("stopPrice".to_string(), stop_price.clone());
        }
        if let Some(ref close_position) = request.close_position {
            params.insert("closePosition".to_string(), close_position.clone());
        }
        if let Some(ref activation_price) = request.activation_price {
            params.insert("activationPrice".to_string(), activation_price.clone());
        }
        if let Some(ref callback_rate) = request.callback_rate {
            params.insert("callbackRate".to_string(), callback_rate.clone());
        }
        if let Some(ref working_type) = request.working_type {
            params.insert("workingType".to_string(), working_type.clone());
        }
        if let Some(ref price_protect) = request.price_protect {
            params.insert("priceProtect".to_string(), price_protect.clone());
        }
        if let Some(ref new_order_resp_type) = request.new_order_resp_type {
            params.insert("newOrderRespType".to_string(), new_order_resp_type.clone());
        }
        if let Some(ref price_match) = request.price_match {
            params.insert("priceMatch".to_string(), price_match.clone());
        }
        if let Some(ref self_trade_prevention_mode) = request.self_trade_prevention_mode {
            params.insert(
                "selfTradePreventionMode".to_string(),
                self_trade_prevention_mode.clone(),
            );
        }
        if let Some(ref good_till_date) = request.good_till_date {
            params.insert("goodTillDate".to_string(), good_till_date.to_string());
        }

        // 添加时间戳和接收窗口
        let timestamp = request.timestamp.unwrap_or_else(Self::get_timestamp);
        params.insert("timestamp".to_string(), timestamp.to_string());
        params.insert(
            "recvWindow".to_string(),
            request.recv_window.unwrap_or(60000).to_string(),
        );

        // 构建查询字符串
        let query_string = self.build_query_string(&params);

        // 生成签名
        let signature = self.generate_signature(&query_string);

        // 构建完整 URL
        let url = format!(
            "{}/order?{}&signature={}",
            self.base_url, query_string, signature
        );

        // 发送请求 - 只使用 URL 参数，不发送 JSON body
        let response = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        // 检查响应状态
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API 请求失败: {}", error_text));
        }

        // 解析响应
        let order_response: OrderResponse = response.json().await?;
        Ok(order_response)
    }

    /// 创建市价买单的便捷方法
    pub async fn market_buy(&self, symbol: &str, quantity: &str) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            quantity: Some(quantity.to_string()),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }

    /// 创建市价卖单的便捷方法
    pub async fn market_sell(&self, symbol: &str, quantity: &str) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Market,
            quantity: Some(quantity.to_string()),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }

    /// 创建限价买单的便捷方法
    pub async fn limit_buy(
        &self,
        symbol: &str,
        quantity: &str,
        price: &str,
    ) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: Some(quantity.to_string()),
            price: Some(price.to_string()),
            time_in_force: Some(TimeInForce::Gtc),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }

    /// 创建限价卖单的便捷方法
    pub async fn limit_sell(
        &self,
        symbol: &str,
        quantity: &str,
        price: &str,
    ) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Limit,
            quantity: Some(quantity.to_string()),
            price: Some(price.to_string()),
            time_in_force: Some(TimeInForce::Gtc),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }
}
