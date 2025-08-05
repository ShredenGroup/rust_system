use crate::common::ts::IsClosed;
use crate::common::ts::Strategy;
use crate::strategy::common::{Signal, SignalType};
use anyhow::Result;
use std::sync::Arc;
use ta::indicators::new_macd::NewMacd;
use ta::{Close, Next};

#[derive(Clone)]
pub struct MacdStrategy {
    pub macd: NewMacd,
    pub last_macd: f64,
    pub last_signal: f64,
}

impl MacdStrategy {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Result<Self> {
        Ok(Self {
            macd: NewMacd::new(fast_period, slow_period, signal_period)?,
            last_macd: 0.0,
            last_signal: 0.0,
        })
    }
}

impl<T> Strategy<&T> for MacdStrategy
where
    T: Close + IsClosed,
{
    type Output = Signal;

    fn on_kline_update(&mut self, input: &T) -> Signal {
        let output = self.macd.next(input);
        let current_macd = output.macd;
        let current_signal = output.signal;

        // 检测金叉和死叉
        let is_golden_cross = self.last_macd <= self.last_signal && current_macd > current_signal;
        let is_death_cross = self.last_macd >= self.last_signal && current_macd < current_signal;

        // 更新上一次的值
        self.last_macd = current_macd;
        self.last_signal = current_signal;

        // 生成交易信号
        if is_golden_cross {
            Signal::buy("BTCUSDT".to_string(), input.close(), 0.01)
        } else if is_death_cross {
            Signal::sell("BTCUSDT".to_string(), input.close(), 0.01)
        } else {
            Signal::hold()
        }
    }

    fn name(&self) -> String {
        "MACD".to_string()
    }
}

impl<T> Strategy<Arc<T>> for MacdStrategy
where
    T: Close + IsClosed + Send + Sync + 'static,
{
    type Output = Signal;

    fn on_kline_update(&mut self, input: Arc<T>) -> Signal {
        self.on_kline_update(input.as_ref())
    }

    fn name(&self) -> String {
        "MACD".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::binance::websocket::KlineInfo;

    #[test]
    fn test_macd_strategy_creation() {
        let strategy = MacdStrategy::new(12, 26, 9);
        assert!(strategy.is_ok());
    }

    #[test]
    fn test_macd_signals() {
        let mut strategy = MacdStrategy::new(12, 26, 9).unwrap();
        
        // 创建一系列价格数据来测试金叉和死叉
        let prices = vec![
            100.0, 102.0, 104.0, 106.0, 108.0, 110.0, // 上升趋势
            109.0, 107.0, 105.0, 103.0, 101.0, 99.0,  // 下降趋势
        ];

        for price in prices {
            let kline = KlineInfo {
                close_price: price,
                // ... 其他字段使用默认值
                start_time: 0,
                close_time: 0,
                symbol: "BTCUSDT".to_string(),
                interval: "1m".to_string(),
                first_trade_id: 0,
                last_trade_id: 0,
                open_price: price,
                high_price: price,
                low_price: price,
                base_volume: 0.0,
                trade_count: 0,
                is_closed: true,
                quote_volume: 0.0,
                taker_buy_base_volume: 0.0,
                taker_buy_quote_volume: 0.0,
                ignore: "".to_string(),
            };

            let signal = strategy.on_kline_update(&kline);
            if signal.is_actionable() {
                println!("Price: {}, Signal: {:?}", price, signal);
            }
        }
    }
}
