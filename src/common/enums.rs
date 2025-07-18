#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum Exchange {
    Binance = 0,
    Okex = 1,
    MEXC = 2,
}

#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum Strategy {
    MACD = 0,
    HBFC = 1,
}
