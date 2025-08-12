#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum Exchange {
    Binance = 0,
    Okex = 1,
    MEXC = 2,
}

#[derive(Debug, Clone, PartialEq,Copy,Hash,Eq)]
#[repr(u8)]
pub enum StrategyName {
    MACD = 0,
    HBFC = 1,
    BOLLINGER = 2,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStutus {
    Pending,
    Success,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PositionSide {
    Long = 1,
    Short = 2,
    NoPosition = 0,
}
