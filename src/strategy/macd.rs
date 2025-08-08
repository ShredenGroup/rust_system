use crate::common::ts::IsClosed;
use crate::common::ts::Strategy;
use crate::common::signal::{TradingSignal, Side, Signal, MarketSignal};
use crate::common::enums::{Exchange, StrategyName};
use crate::common::utils::get_timestamp_ms;
use anyhow::Result;
use std::sync::Arc;
use ta::indicators::new_macd::NewMacd;
use ta::{Close, Next, Update};
use crate::common::consts::*;
#[derive(Clone)]
pub struct MacdStrategy {
    pub macd: NewMacd,
    pub last_macd: f64,
    pub last_signal: f64,
    pub count: usize,
    finish_init:bool,
    slow_period: usize,  // 添加 slow_period 字段
}

impl MacdStrategy {
    pub fn new(fast_period: usize, slow_period: usize, signal_period: usize) -> Result<Self> {
        Ok(Self {
            macd: NewMacd::new(fast_period, slow_period, signal_period)?,
            last_macd: 0.0,
            last_signal: 0.0,
            count: 0,
            finish_init:false,
            slow_period,  // 保存 slow_period

        })
    }
}

impl<T> Strategy<&T> for MacdStrategy
where
    T: Close + IsClosed,
{
    type Output = Option<TradingSignal>;

    fn on_kline_update(&mut self, input: &T) -> Option<TradingSignal> {
        // 如果已经完成初始化，执行正常逻辑
        if self.finish_init {
            let output = if input.is_closed() {
                let output = self.macd.next(input);
                println!("插入最新的K线");
                // 只在收盘时更新last值
                self.last_macd = output.macd;
                self.last_signal = output.signal;
                output
            } else {
                // 未收盘时使用update
                println!("未收盘，更新macd的值");
                self.macd.update(input.close())
            };

            let current_macd = output.macd;
            let current_signal = output.signal;

            // 检测金叉和死叉
            let is_golden_cross = self.last_macd <= self.last_signal && current_macd > current_signal;
            let is_death_cross = self.last_macd >= self.last_signal && current_macd < current_signal;

            if is_golden_cross {
                Some(TradingSignal::new_market_signal(
                    1,                      // hardcoded id
                    BTC_USDT_SYMBOL.to_string(), 
                    Side::Buy,             // 金叉买入
                    StrategyName::MACD,
                    0.01,                  // 固定数量
                    Exchange::Binance,
                    get_timestamp_ms() as u32,
                    None,                  // 不设止盈
                    None,                  // 不设止损
                    input.close(),         // 当前价格
                ))
            } else if is_death_cross {
                Some(TradingSignal::new_market_signal(
                    1,                      // hardcoded id
                    "BTCUSDT".to_string(),
                    Side::Sell,            // 死叉卖出
                    StrategyName::MACD,
                    0.01,                  // 固定数量
                    Exchange::Binance,
                    get_timestamp_ms() as u32,
                    None,                  // 不设止盈
                    None,                  // 不设止损
                    input.close(),         // 当前价格
                ))
            } else {
                None  // 没有交叉，不生成信号
            }
        } else {
            // 未完成初始化，只进行数据收集
            if self.count < self.slow_period {
                let output=self.macd.next(input);
                self.count += 1;
                println!("初始化中: {}/{}", self.count, self.slow_period);
                
                // 如果达到所需数量，标记为初始化完成
                if self.count == self.slow_period {
                    self.last_macd=output.macd;
                    self.last_signal=output.signal;
                    println!("初始化倒数第二阶段，获得last_macd以及last_signal，{}/{}",self.last_macd,self.last_signal);
                }
            }else{
                self.macd.next(input);
                self.finish_init=true;
                self.count+=1;
                println!("初始化完成，开始正常运行");
            }
            None
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
    type Output = Option<TradingSignal>;

    fn on_kline_update(&mut self, input: Arc<T>) -> Option<TradingSignal> {
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
        
        let prices = vec![
            100.0, 102.0, 104.0, 106.0, 108.0, 110.0, // 上升趋势
            109.0, 107.0, 105.0, 103.0, 101.0, 99.0,  // 下降趋势
        ];

        for (i, price) in prices.iter().enumerate() {
            let kline = KlineInfo {
                close_price: *price,
                start_time: 0,
                close_time: 0,
                symbol: "BTCUSDT".to_string(),
                interval: "1m".to_string(),
                first_trade_id: 0,
                last_trade_id: 0,
                open_price: *price,
                high_price: *price,
                low_price: *price,
                base_volume: 0.0,
                trade_count: 0,
                is_closed: true,
                quote_volume: 0.0,
                taker_buy_base_volume: 0.0,
                taker_buy_quote_volume: 0.0,
                ignore: "".to_string(),
            };

            if let Some(trading_signal) = strategy.on_kline_update(&kline) {
                println!("K线 #{}, 价格: {}, 计数: {}, 产生信号!", i + 1, price, strategy.count);
            } else {
                println!("K线 #{}, 价格: {}, 计数: {}, 无信号", i + 1, price, strategy.count);
            }
        }
    }
}