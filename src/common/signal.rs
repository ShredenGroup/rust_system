use super::enums::{Exchange, StrategyName};
use super::utils::get_timestamp_ms;
use super::ts::SignalTs;
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum Side {
    Sell = 0,  // 显式指定值
    Buy = 1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PositionSide {
    Long = 0,  // 显式指定值
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
            // 修复：移除多余的 quantity 参数
            signal: Signal::Market(MarketSignal::new(side, stop_price, profit_price)),
            latest_price,
            exchange,
            data_timestamp,
            timestamp: get_timestamp_ms(),
            strategy,
        }
    }
    
    // 添加新的生成平仓信号的方法
    pub fn new_close_signal(
        id: u32,
        symbol: String,
        current_position: u8,  // 0: 无仓位, 1: 多头, 2: 空头
        strategy: StrategyName,
        quantity: f64,
        exchange: Exchange,
        latest_price: f64,
    ) -> Self {
        // 根据当前持仓方向决定平仓方向
        let close_side = match current_position {
            1 => Side::Sell,  // 持有多头，需要卖出平仓
            2 => Side::Buy,   // 持有空头，需要买入平仓
            _ => panic!("Unexpected state: generating close signal without position"),
        };

        // 创建平仓信号，确保 is_closed = true
        let mut market_signal = MarketSignal::new(close_side, None, None);
        market_signal.is_closed = true;  // 明确设置为平仓信号

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
}
impl SignalTs for TradingSignal{
    fn signal_strategy(&self) -> StrategyName {
        self.strategy
    }
}
impl MarketSignal {
    pub fn new(
        side: Side,
        stop_price: Option<f64>,
        profit_price: Option<f64>,
    ) -> Self {
        Self {
            side,
            stop_price,
            profit_price,
            is_closed: false,
        }
    }
    pub fn simple_new(side:Side) -> Self{
        Self::new(side, None, None)
    }
}

impl LimitSignal {
    pub fn new(side: Option<PositionSide>, price: f64) -> Self {
        Self {
            side,
            price,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // 辅助函数：创建一个基础的市场信号
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
        
        // 检查 MarketSignal 的内部状态
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
            Some(2000.0),  // profit price
            Some(1800.0),  // stop price
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
        assert_eq!(Side::Buy as u8, 1);    // 期望 Buy = 1
        assert_eq!(Side::Sell as u8, 0);   // 期望 Sell = 0
        
        assert_eq!(PositionSide::Long as u8, 0);   // 期望 Long = 0
        assert_eq!(PositionSide::Short as u8, 1);  // 期望 Short = 1
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
        // 修复：添加括号明确运算优先级
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
        
        // 假设你还支持其他交易所
        // let mexc_signal = TradingSignal::new_market_signal(
        //     2,
        //     "BTCUSDT".to_string(),
        //     Side::Buy,
        //     StrategyName::MACD,
        //     1.0,
        //     Exchange::Mexc,
        //     get_timestamp_ms() as u32,
        //     None,
        //     None,
        //     50000.0,
        // );
        // assert_eq!(mexc_signal.exchange, Exchange::Mexc);
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
        
        // 确保重要字段都在debug输出中
        assert!(debug_output.contains("BTCUSDT"));
        assert!(debug_output.contains("Buy"));
        assert!(debug_output.contains("MACD"));
    }

    // 属性测试：如果你使用 proptest crate
    #[cfg(feature = "proptest")]
    use proptest::prelude::*;

    #[cfg(feature = "proptest")]
    proptest! {
        #[test]
        fn test_market_signal_props(
            id in 0u32..1000,
            quantity in 0.0f64..100.0,
            price in 1000.0f64..100000.0
        ) {
            let signal = TradingSignal::new_market_signal(
                id,
                "BTCUSDT".to_string(),
                Side::Buy,
                StrategyName::MACD,
                quantity,
                Exchange::Binance,
                get_timestamp_ms() as u32,
                Some(price * 1.1),  // profit price 10% higher
                Some(price * 0.9),  // stop price 10% lower
                price,
            );
            
            prop_assert!(signal.id == id);
            prop_assert!(signal.quantity == quantity);
            prop_assert!(signal.latest_price == price);
        }
    }
}
