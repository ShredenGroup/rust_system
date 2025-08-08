#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum Exchange {
    Binance = 0,
    Okex = 1,
    MEXC = 2,
}

#[derive(Debug, Clone, PartialEq,Copy)]
#[repr(u8)]
pub enum StrategyName {
    MACD = 0,
    HBFC = 1,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OrderStutus {
    Pending,
    Success,
    Failed,
    Canceled,
}
