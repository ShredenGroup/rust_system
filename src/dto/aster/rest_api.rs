use serde::{Deserialize, Serialize};

/// 订单类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// 下单请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    #[serde(rename = "type")]
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
    pub activation_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_protect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_order_resp_type: Option<String>,
}

// 实现 Default trait
impl Default for OrderRequest {
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
            activation_price: None,
            callback_rate: None,
            working_type: None,
            price_protect: None,
            new_order_resp_type: None,
        }
    }
}

/// 下单响应
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    #[serde(default)]
    pub client_order_id: Option<String>,
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
    #[serde(default)]
    pub stop_price: String,
    pub symbol: String,
    pub time_in_force: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub orig_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activate_price: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_rate: Option<String>,
    pub update_time: i64,
    pub working_type: String,
    pub price_protect: bool,
}

/// ASTER API错误响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsterErrorResponse {
    pub code: i32,
    pub msg: String,
}

/// 批量订单响应项 - 可能是成功订单或错误
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BatchOrderResponseItem {
    Success(OrderResponse),
    Error(AsterErrorResponse),
}

/// 批量订单处理结果
#[derive(Debug, Clone)]
pub struct BatchOrderResult {
    pub successful_orders: Vec<OrderResponse>,
    pub failed_orders: Vec<(usize, AsterErrorResponse)>, // (原始索引, 错误)
    pub total_requested: usize,
}

impl BatchOrderResult {
    pub fn new(total: usize) -> Self {
        Self {
            successful_orders: Vec::new(),
            failed_orders: Vec::new(),
            total_requested: total,
        }
    }

    pub fn is_all_failed(&self) -> bool {
        self.successful_orders.is_empty() && !self.failed_orders.is_empty()
    }

    pub fn is_all_success(&self) -> bool {
        self.failed_orders.is_empty() && self.successful_orders.len() == self.total_requested
    }

    pub fn has_partial_success(&self) -> bool {
        !self.successful_orders.is_empty() && !self.failed_orders.is_empty()
    }
}

