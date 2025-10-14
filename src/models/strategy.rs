pub enum StrategyType {
    Bollinger,
    Macd,
    Q1,
}
pub struct StrategySetting {
    pub strategy_type: StrategyType,
    pub risk_amount:Option<u64>,
    pub risk_percentage:Option<u64>,
    pub leverage:Option<u64>,
}

impl StrategySetting {
    pub fn new(strategy_type: StrategyType, risk_amount:Option<u64>, risk_percentage:Option<u64>, leverage:Option<u64>) -> Self {
        Self {
            strategy_type,
            risk_amount,
            risk_percentage,
            leverage,
        }
    }
}
