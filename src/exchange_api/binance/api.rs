use crate::common::consts::BINANCE_FUTURES_URL;
use crate::common::utils::generate_hmac_signature;
use anyhow::{Ok, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// 订单类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderType {
    Limit,
    Market,
    Stop,
    StopMarket,
    TakeProfit,
    TakeProfitMarket,
    TrailingStopMarket,
}

/// 订单方向枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

/// 时间强制类型枚举
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeInForce {
    Gtc, // Good Till Cancel
    Ioc, // Immediate or Cancel
    Fok, // Fill or Kill
    Gtx, // Good Till Crossing
    Gtd, // Good Till Date
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewKlineRequest {
    pub symbol: String,
    pub interval: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineData {
    #[serde(rename = "0")]
    pub open_time: i64,
    #[serde(rename = "1")]
    pub open: String,
    #[serde(rename = "2")]
    pub high: String,
    #[serde(rename = "3")]
    pub low: String,
    #[serde(rename = "4")]
    pub close: String,
    #[serde(rename = "5")]
    pub volume: String,
    #[serde(rename = "6")]
    pub close_time: i64,
    #[serde(rename = "7")]
    pub quote_volume: String,
    #[serde(rename = "8")]
    pub trades_count: i64,
    #[serde(rename = "9")]
    pub taker_buy_volume: String,
    #[serde(rename = "10")]
    pub taker_buy_quote_volume: String,
    #[serde(rename = "11")]
    pub ignore: String,
}

pub type NewKlineResponse = Vec<KlineData>;

/// 下单请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,

    // 可选参数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_side: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_in_force: Option<TimeInForce>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reduce_only: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_client_order_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub close_position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_protect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_order_resp_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_match: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub self_trade_prevention_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub good_till_date: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recv_window: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<u64>,
}

/// 下单响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewOrderResponse {
    pub client_order_id: String,
    pub cum_qty: String,
    pub cum_quote: String,
    pub executed_qty: String,
    pub order_id: i64,
    pub avg_price: String,
    pub orig_qty: String,
    pub price: String,
    pub reduce_only: bool,
    pub side: String,
    pub position_side: String,
    pub status: String,
    pub stop_price: String,
    pub close_position: bool,
    pub symbol: String,
    pub time_in_force: String,
    pub order_type: String,
    pub orig_type: String,
    pub activate_price: Option<String>,
    pub price_rate: Option<String>,
    pub update_time: i64,
    pub working_type: String,
    pub price_protect: bool,
    pub price_match: String,
    pub self_trade_prevention_mode: String,
    pub good_till_date: Option<i64>,
}

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

    pub async fn get_klines(&self, request: &NewKlineRequest) -> Result<NewKlineResponse> {
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
    pub async fn new_order(&self, request: NewOrderRequest) -> Result<NewOrderResponse> {
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
        let order_response: NewOrderResponse = response.json().await?;
        Ok(order_response)
    }

    /// 创建市价买单的便捷方法
    pub async fn market_buy(&self, symbol: &str, quantity: &str) -> Result<NewOrderResponse> {
        let request = NewOrderRequest {
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
    pub async fn market_sell(&self, symbol: &str, quantity: &str) -> Result<NewOrderResponse> {
        let request = NewOrderRequest {
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
    ) -> Result<NewOrderResponse> {
        let request = NewOrderRequest {
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
    ) -> Result<NewOrderResponse> {
        let request = NewOrderRequest {
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

// 为 NewOrderRequest 实现 Default trait
impl Default for NewOrderRequest {
    fn default() -> Self {
        Self {
            symbol: String::new(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            position_side: None,
            time_in_force: None,
            quantity: None,
            reduce_only: None,
            price: None,
            new_client_order_id: None,
            stop_price: None,
            close_position: None,
            activation_price: None,
            callback_rate: None,
            working_type: None,
            price_protect: None,
            new_order_resp_type: None,
            price_match: None,
            self_trade_prevention_mode: None,
            good_till_date: None,
            recv_window: None,
            timestamp: None,
        }
    }
}

impl NewKlineRequest {
    pub fn to_params(&self) -> Result<HashMap<String, String>> {
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), self.symbol.clone());
        params.insert("interval".to_string(), self.interval.clone());
        if let Some(ref start_time) = self.start_time {
            params.insert("startTime".to_string(), start_time.clone());
        }
        if let Some(ref end_time) = self.end_time {
            params.insert("endTime".to_string(), end_time.clone());
        }
        if let Some(ref limit) = self.limit {
            params.insert("limit".to_string(), limit.clone());
        }
        Ok(params)
    }
}
