// 优化后的信号类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SignalType {
    Buy = 1,
    Sell = 0,
}

#[derive(Debug, Clone)]
pub struct Signal {
    pub signal_type: Option<SignalType>,  // None 表示 Hold
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub timestamp: i64,
}

impl Signal {
    pub fn buy(symbol: String, price: f64, quantity: f64) -> Self {
        Self {
            signal_type: Some(SignalType::Buy),
            symbol,
            price,
            quantity,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn sell(symbol: String, price: f64, quantity: f64) -> Self {
        Self {
            signal_type: Some(SignalType::Sell),
            symbol,
            price,
            quantity,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn hold() -> Self {
        Self {
            signal_type: None,
            symbol: String::new(),
            price: 0.0,
            quantity: 0.0,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
    
    pub fn is_actionable(&self) -> bool {
        self.signal_type.is_some()
    }
    
    // 获取数字表示，方便传输
    pub fn to_u8(&self) -> Option<u8> {
        self.signal_type.map(|t| t as u8)
    }
    
    // 从数字创建信号类型
    pub fn from_u8(value: u8) -> Option<SignalType> {
        match value {
            1 => Some(SignalType::Buy),
            0 => Some(SignalType::Sell),
            _ => None,
        }
    }
}
