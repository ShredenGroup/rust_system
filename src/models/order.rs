use crate::common::enums::{OrderStutus, StrategyName};

pub struct Order {
    pub symbol: String,
    pub amount: f64,
    pub strategy: StrategyName,
    pub order_id: u64,
    pub status: OrderStutus,
    pub timestamp: u64,
    pub updated_timestamp: u64,
}

impl Order {}
