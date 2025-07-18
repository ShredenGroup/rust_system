use super::enums::{Exchange, Strategy};
use super::utils::get_timestamp_ms;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Side {
    Buy = 1,
    Sell = 0,
    CloseLong = 2,
    CloseShort = 3,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PositionSide {
    Long = 0,
    Short = 1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarketSignal {
    pub side: Side,
    pub quantity: f64,
    pub stop_price: Option<f64>,
    pub profit_price: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LimitSignal {
    pub side: Side,
    pub quantity: f64,
    pub price: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    Market(MarketSignal),
    Limit(LimitSignal),
}
#[derive(Debug, Clone, PartialEq)]
struct TradingSignal {
    id: u32,
    symbol: String,
    strategy: Strategy,
    signal: Signal,
    latest_price: f64,
    exchange: Exchange,
    data_timestamp: u32,
    timestamp: u64,
}
impl TradingSignal {
    pub fn new_market_signal(
        id: u32,
        symbol: String,
        side: Side,
        strategy: Strategy,
        quantity: f64,
        exchange: Exchange,
        data_timestamp: u32,
        profit_price: Option<f64>,
        stop_price: Option<f64>,
        latest_price: f64,
    ) -> Self {
        Self {
            id,
            symbol,
            signal: Signal::Market(MarketSignal::new(side, quantity, stop_price, profit_price)),
            latest_price,
            exchange,
            data_timestamp,
            timestamp: get_timestamp_ms(),
            strategy,
        }
    }
}
impl MarketSignal {
    pub fn new(
        side: Side,
        quantity: f64,
        stop_price: Option<f64>,
        profit_price: Option<f64>,
    ) -> Self {
        Self {
            side,
            quantity,
            stop_price,
            profit_price,
        }
    }
}

impl LimitSignal {
    pub fn new(side: Side, quantity: f64, price: f64) -> Self {
        Self {
            side,
            quantity,
            price,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_trading_market_signal() {
        let new_trading_signal = TradingSignal::new_market_signal(
            1,
            "BTCUSDT".to_string(),
            Side::Buy,
            Strategy::MACD,
            1000.0,
            Exchange::Binance,
            10000000,
            None,
            None,
            123.0,
        );
        assert_eq!(new_trading_signal.id, 1);
    }
}
