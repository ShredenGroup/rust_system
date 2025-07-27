use crate::common::enums::{Exchange, OrderStutus, StrategyName};
use crate::common::signal::TradingSignal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
pub struct OpenPosition {
    pub position: HashMap<String, u64>,
    pub exchange: Exchange,
    pub strategy_name: StrategyName,
}
pub struct OrderManager {
    pub orders: Vec<Order>,
    pub open_position: Arc<RwLock<HashMap<String, f64>>>,
}
pub struct Order {
    pub exchange: Exchange,
    pub symbol: String,
    pub amount: f64,
    pub strategy: StrategyName,
    pub order_id: u64,
    pub status: OrderStutus,
    pub timestamp: u64,
    pub updated_timestamp: u64,
}

impl Order {}
