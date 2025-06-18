use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

// 预定义的 Stream 键名
pub const ORDER_STREAM: &str = "orders";
pub const MARKET_DATA_STREAM: &str = "market_data";
pub const SIGNAL_STREAM: &str = "signals";
pub const TRADE_STREAM: &str = "trades";

// 预定义的消费者组名
pub const ORDER_PROCESSOR_GROUP: &str = "order_processors";
pub const DATA_AGGREGATOR_GROUP: &str = "data_aggregators";
pub const STRATEGY_GROUP: &str = "strategy_processors";
pub const TRADE_EXECUTOR_GROUP: &str = "trade_executors";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderMessage {
    pub order_id: String,
    pub symbol: String,
    pub side: String, // "BUY" or "SELL"
    pub quantity: f64,
    pub price: Option<f64>, // None for market orders
    pub order_type: String, // "MARKET" or "LIMIT"
    pub timestamp: DateTime<Utc>,
    pub user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataMessage {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMessage {
    pub strategy_id: String,
    pub symbol: String,
    pub signal: String, // "BUY", "SELL", "HOLD"
    pub confidence: f64, // 0.0 to 1.0
    pub timestamp: DateTime<Utc>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeMessage {
    pub trade_id: String,
    pub order_id: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
    pub status: String, // "EXECUTED", "FAILED", "CANCELLED"
} 