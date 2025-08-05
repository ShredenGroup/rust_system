use crate::common::enums::{Exchange, OrderStutus, StrategyName};
use crate::common::signal::TradingSignal;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type TokenName=String;

pub struct OpenPosition {
    pub position: HashMap<TokenName, u64>,
    pub exchange: Exchange,
    pub strategy_name: StrategyName,
}
pub struct OrderManager {
    pub orders: Vec<Order>,
    pub open_position: Arc<RwLock<HashMap<(Exchange,TokenName), f64>>>,
}
pub struct Order {
    pub order_id: usize,
    pub exchange: Exchange,
    pub symbol: String,
    pub amount: f64,
    pub strategy: StrategyName,
    pub status: OrderStutus,
    pub timestamp: u64,
    pub updated_timestamp: u64,
}
pub struct SignalManager{
    pub signal_checkers:Vec<SignalChecker>,
}

pub struct SignalChecker{
    pub signal:TradingSignal,
    pub strategy:StrategyName,
    
}