use crate::common::enums::{Exchange, StrategyName};
use crate::common::ts::SignalTs;
use crate::common::utils::get_timestamp_ms;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Side {
    Sell = 0, // 显式指定值
    Buy = 1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PositionSide {
    Long = 0, // 显式指定值
    Short = 1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MarketSignal {
    pub side: Side,
    pub stop_price: Option<f64>,
    pub profit_price: Option<f64>,
    pub is_closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LimitSignal {
    pub side: Option<PositionSide>,
    pub price: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    Market(MarketSignal),
    Limit(LimitSignal),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TradingSignal {
    pub id: u32,
    pub symbol: String,
    pub strategy: StrategyName,
    pub quantity: f64,
    pub signal: Signal,
    pub side: Side,
    pub latest_price: f64,
    exchange: Exchange,
    data_timestamp: u32,
    timestamp: u64,
}

impl TradingSignal {
    pub fn new_market_signal(
        id: u32,
        symbol: String,
        side: Side,
        strategy: StrategyName,
        quantity: f64,
        exchange: Exchange,
        data_timestamp: u32,
        profit_price: Option<f64>,
        stop_price: Option<f64>,
        latest_price: f64,
    ) -> Self {
        Self {
            id,
            quantity,
            symbol,
            side,
            signal: Signal::Market(MarketSignal::new(side, stop_price, profit_price)),
            latest_price,
            exchange,
            data_timestamp,
            timestamp: get_timestamp_ms(),
            strategy,
        }
    }

    pub fn new_close_signal(
        id: u32,
        symbol: String,
        current_position: u8, // 0: 无仓位, 1: 多头, 2: 空头
        strategy: StrategyName,
        quantity: f64,
        exchange: Exchange,
        latest_price: f64,
    ) -> Self {
        let close_side = match current_position {
            1 => Side::Sell, // 持有多头，需要卖出平仓
            2 => Side::Buy,  // 持有空头，需要买入平仓
            _ => panic!("Unexpected state: generating close signal without position"),
        };

        let mut market_signal = MarketSignal::new(close_side, None, None);
        market_signal.is_closed = true; // 明确设置为平仓信号

        Self {
            id,
            symbol,
            strategy,
            quantity,
            side: close_side,
            signal: Signal::Market(market_signal),
            latest_price,
            exchange,
            data_timestamp: get_timestamp_ms() as u32,
            timestamp: get_timestamp_ms(),
        }
    }
    
    /// 获取交易所
    pub fn exchange(&self) -> Exchange {
        self.exchange
    }
    
    /// 获取时间戳
    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

impl SignalTs for TradingSignal {
    fn signal_strategy(&self) -> StrategyName {
        self.strategy
    }
}

impl MarketSignal {
    pub fn new(side: Side, stop_price: Option<f64>, profit_price: Option<f64>) -> Self {
        Self {
            side,
            stop_price,
            profit_price,
            is_closed: false,
        }
    }

    pub fn simple_new(side: Side) -> Self {
        Self::new(side, None, None)
    }
}

impl LimitSignal {
    pub fn new(side: Option<PositionSide>, price: f64) -> Self {
        Self { side, price }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_basic_market_signal() -> TradingSignal {
        TradingSignal::new_market_signal(
            1,
            "BTCUSDT".to_string(),
            Side::Buy,
            StrategyName::MACD,
            1.0,
            Exchange::Binance,
            get_timestamp_ms() as u32,
            None,
            None,
            50000.0,
        )
    }

    #[test]
    fn test_market_signal_basic() {
        let signal = create_basic_market_signal();

        assert_eq!(signal.id, 1);
        assert_eq!(signal.symbol, "BTCUSDT");
        assert_eq!(signal.side, Side::Buy);
        assert_eq!(signal.quantity, 1.0);
        assert_eq!(signal.exchange, Exchange::Binance);
        assert_eq!(signal.latest_price, 50000.0);

        if let Signal::Market(market_signal) = signal.signal {
            assert_eq!(market_signal.side, Side::Buy);
            assert_eq!(market_signal.stop_price, None);
            assert_eq!(market_signal.profit_price, None);
            assert_eq!(market_signal.is_closed, false);
        } else {
            panic!("Expected Market signal");
        }
    }

    #[test]
    fn test_market_signal_with_stop_profit() {
        let signal = TradingSignal::new_market_signal(
            1,
            "ETHUSDT".to_string(),
            Side::Sell,
            StrategyName::MACD,
            2.0,
            Exchange::Binance,
            get_timestamp_ms() as u32,
            Some(2000.0), // profit price
            Some(1800.0), // stop price
            1900.0,
        );

        if let Signal::Market(market_signal) = signal.signal {
            assert_eq!(market_signal.stop_price, Some(1800.0));
            assert_eq!(market_signal.profit_price, Some(2000.0));
        }
    }

    #[test]
    fn test_limit_signal() {
        let limit_signal = LimitSignal::new(Some(PositionSide::Long), 45000.0);

        assert_eq!(limit_signal.side, Some(PositionSide::Long));
        assert_eq!(limit_signal.price, 45000.0);
    }

    #[test]
    fn test_side_conversion() {
        assert_eq!(Side::Buy as u8, 1); // 期望 Buy = 1
        assert_eq!(Side::Sell as u8, 0); // 期望 Sell = 0

        assert_eq!(PositionSide::Long as u8, 0); // 期望 Long = 0
        assert_eq!(PositionSide::Short as u8, 1); // 期望 Short = 1
    }

    #[test]
    fn test_market_signal_simple_new() {
        let simple_signal = MarketSignal::simple_new(Side::Buy);

        assert_eq!(simple_signal.side, Side::Buy);
        assert_eq!(simple_signal.stop_price, None);
        assert_eq!(simple_signal.profit_price, None);
        assert_eq!(simple_signal.is_closed, false);
    }

    #[test]
    fn test_signal_timestamps() {
        let signal = create_basic_market_signal();

        assert!(signal.timestamp > 0);
        assert!(signal.data_timestamp > 0);
        let now = get_timestamp_ms();
        assert!((now - signal.timestamp as u64) < 1000); // 在1秒内
    }

    #[test]
    fn test_different_exchanges() {
        let binance_signal = TradingSignal::new_market_signal(
            1,
            "BTCUSDT".to_string(),
            Side::Buy,
            StrategyName::MACD,
            1.0,
            Exchange::Binance,
            get_timestamp_ms() as u32,
            None,
            None,
            50000.0,
        );

        assert_eq!(binance_signal.exchange, Exchange::Binance);
    }

    #[test]
    fn test_signal_clone() {
        let original = create_basic_market_signal();
        let cloned = original.clone();

        assert_eq!(original, cloned);
    }

    #[test]
    fn test_signal_debug_print() {
        let signal = create_basic_market_signal();
        let debug_output = format!("{:?}", signal);

        assert!(debug_output.contains("BTCUSDT"));
        assert!(debug_output.contains("Buy"));
        assert!(debug_output.contains("MACD"));
    }
}
