use ta::indicators::{AverageTrueRange, new_bollinger::NewBollinger};
use ta::{Next, Close, High, Low, Open, Update};
use crate::common::enums::{Exchange, StrategyName};
use crate::common::signal::{TradingSignal, Side, Signal, MarketSignal};
use crate::common::ts::{Strategy, IsClosed};
use crate::common::consts::*;
use crate::common::utils::get_timestamp_ms;
use anyhow::Result;

#[derive(Clone)]
pub struct BollingerStrategy {
    pub bollinger: NewBollinger,
    pub atr: AverageTrueRange,
    pub count: usize,
    pub finish_init: bool,
    period: usize,
    last_price: f64,
    pub last_upper: f64,
    pub last_lower: f64,
    pub last_middle: f64,
    pub last_atr: f64,
    pub current_signal: u8,  // 0: 已平仓, 1: 多头, 2: 空头
}

impl BollingerStrategy {
    pub fn new(period: usize, std_dev: f64) -> Result<Self> {
        Ok(Self {
            bollinger: NewBollinger::new(period, std_dev)?,
            atr: AverageTrueRange::new(20)?,
            count: 0,
            finish_init: false,
            period,
            last_price: 0.0,
            last_upper: 0.0,
            last_lower: 0.0,
            last_middle: 0.0,
            last_atr: 0.0,
            current_signal: 0,
        })
    }

    fn check_signals(&mut self, close_price: f64, upper_band: f64, lower_band: f64, middle_band: f64, atr_value: f64) -> Option<TradingSignal> {
        // 1. 检查是否需要平仓（如果有持仓且价格触及中线）
        if self.current_signal != 0 {
            // 根据持仓方向判断是否需要平仓
            let should_close = match self.current_signal {
                1 => self.last_price < middle_band && close_price >= middle_band, // 多头穿过中线
                2 => self.last_price > middle_band && close_price <= middle_band, // 空头穿过中线
                _ => false,
            };

            if should_close {
                let position_to_close = self.current_signal;  // 保存要平仓的位置
                self.current_signal = 0;  // 重置信号状态
                self.last_price = close_price;
                
                // 创建平仓信号并标记为平仓操作
                let signal = TradingSignal::new_close_signal(
                    1,
                    TURBO_USDT_SYMBOL.to_string(),
                    position_to_close,  // 使用保存的位置
                    StrategyName::BOLLINGER,
                    10000.0,
                    Exchange::Binance,
                    close_price,
                );
                
                return Some(signal);
            }
        }

        // 2. 检查是否需要开新仓
        if self.current_signal == 0 {  // 只有在没有持仓时才开新仓
            if close_price >= upper_band {
                // 触及上轨，做空
                let stop_price = close_price + (2.0 * atr_value);
                // 限制止损价格精度，避免币安精度错误
                let stop_price = (stop_price * 1000000.0).round() / 1000000.0; // 限制到6位小数
                
                self.current_signal = 2;  // 设置为空头状态
                self.last_price = close_price;
                return Some(TradingSignal::new_market_signal(
                    1,
                    TURBO_USDT_SYMBOL.to_string(),
                    Side::Sell,
                    StrategyName::BOLLINGER,
                    10000.0,
                    Exchange::Binance,
                    get_timestamp_ms() as u32,
                    None,
                    Some(stop_price),
                    close_price,
                ));
            } else if close_price <= lower_band {
                // 触及下轨，做多
                let stop_price = close_price - (2.0 * atr_value);
                // 限制止损价格精度，避免币安精度错误
                let stop_price = (stop_price * 1000000.0).round() / 1000000.0; // 限制到6位小数
                
                self.current_signal = 1;  // 设置为多头状态
                self.last_price = close_price;
                
                return Some(TradingSignal::new_market_signal(
                    1,
                    TURBO_USDT_SYMBOL.to_string(),
                    Side::Buy,
                    StrategyName::BOLLINGER,
                    10000.0,
                    Exchange::Binance,
                    get_timestamp_ms() as u32,
                    None,  // 止盈价格
                    Some(stop_price),  // 止损价格
                    close_price,
                ));
            }
        }

        None
    }
}

impl<T> Strategy<&T> for BollingerStrategy
where
    T: High + Low + Close + Open + IsClosed,
{
    type Output = Option<TradingSignal>;

    fn on_kline_update(&mut self, input: &T) -> Self::Output {
        if self.finish_init {
            let close_price = input.close();
            
            if input.is_closed() {
                // K线已收盘，使用next方法更新指标
                let bb_output = self.bollinger.next(input);
                let atr_value = self.atr.next(input);
                
                // 更新最新的布林带值和ATR值
                self.last_upper = bb_output.upper;
                self.last_lower = bb_output.lower;
                self.last_middle = bb_output.average;
                self.last_atr = atr_value;

                // 检查信号
                let signal = self.check_signals(close_price, bb_output.upper, bb_output.lower, bb_output.average, atr_value);
                
                // 更新最新价格
                self.last_price = close_price;
                signal
            } else {
                // K线未收盘，使用上次的布林带值，ATR保持上次的值
                // 注意：标准BollingerBands没有update方法，只能使用next方法
                // 在未收盘时，我们使用上次计算的值进行信号检测
                
                // 使用上次的ATR值进行信号检测
                let signal = self.check_signals(close_price, self.last_upper, self.last_lower, self.last_middle, self.last_atr);
                
                // 更新最新价格
                self.last_price = close_price;
                signal
            }
        } else {
            // 初始化阶段
            if self.count < self.period {
                let bb_output = self.bollinger.next(input);
                let atr_value = self.atr.next(input);
                self.count += 1;
                println!("布林带初始化中: {}/{}", self.count, self.period);
                
                if self.count == self.period {
                    self.finish_init = true;
                    self.last_price = input.close();
                    self.last_upper = bb_output.upper;
                    self.last_lower = bb_output.lower;
                    self.last_middle = bb_output.average;
                    self.last_atr = atr_value;
                    println!("布林带初始化完成，开始正常运行");
                }
            }
            None
        }
    }

    fn name(&self) -> String {
        "BOLLINGER".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::binance::websocket::KlineInfo;

    #[test]
    fn test_bollinger_strategy_creation() {
        let strategy = BollingerStrategy::new(20, 2.0);
        assert!(strategy.is_ok());
    }

    #[test]
    fn test_bollinger_signals() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        
        // 创建模拟数据进行测试
        for i in 0..30 {
            let price = 100.0 + (i as f64 * 2.0); // 模拟上涨趋势
            let kline = KlineInfo {
                close_price: price,
                high_price: price + 1.0,
                low_price: price - 1.0,
                open_price: price - 0.5,
                start_time: 0,
                close_time: 0,
                symbol: "BTCUSDT".to_string(),
                interval: "1m".to_string(),
                first_trade_id: 0,
                last_trade_id: 0,
                base_volume: 0.0,
                trade_count: 0,
                is_closed: true,
                quote_volume: 0.0,
                taker_buy_base_volume: 0.0,
                taker_buy_quote_volume: 0.0,
                ignore: "".to_string(),
            };

            if let Some(signal) = strategy.on_kline_update(&kline) {
                println!(
                    "K线 #{}, 价格: {}, 产生信号: {:?}", 
                    i + 1, 
                    price, 
                    signal.side
                );
            }
        }
    }

    #[test]
    fn test_bollinger_close_long_position() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        
        // 1. 先初始化策略
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1);
            let kline = KlineInfo {
                close_price: price,
                high_price: price + 0.05,
                low_price: price - 0.05,
                open_price: price - 0.02,
                start_time: 0,
                close_time: 0,
                symbol: "TURBOUSDT".to_string(),
                interval: "1m".to_string(),
                first_trade_id: 0,
                last_trade_id: 0,
                base_volume: 0.0,
                trade_count: 0,
                is_closed: true,
                quote_volume: 0.0,
                taker_buy_base_volume: 0.0,
                taker_buy_quote_volume: 0.0,
                ignore: "".to_string(),
            };
            strategy.on_kline_update(&kline);
        }
        
        // 2. 模拟做多信号（触及下轨）
        let long_kline = KlineInfo {
            close_price: 95.0,  // 触及下轨
            high_price: 95.5,
            low_price: 94.5,
            open_price: 95.2,
            start_time: 0,
            close_time: 0,
            symbol: "TURBOUSDT".to_string(),
            interval: "1m".to_string(),
            first_trade_id: 0,
            last_trade_id: 0,
            base_volume: 0.0,
            trade_count: 0,
            is_closed: true,
            quote_volume: 0.0,
            taker_buy_base_volume: 0.0,
            taker_buy_quote_volume: 0.0,
            ignore: "".to_string(),
        };
        
        // 应该产生做多信号
        if let Some(signal) = strategy.on_kline_update(&long_kline) {
            println!("做多信号: {:?}", signal.side);
            assert_eq!(signal.side, Side::Buy);
            assert_eq!(strategy.current_signal, 1); // 多头状态
        } else {
            panic!("应该产生做多信号");
        }
        
        // 3. 模拟价格穿过中线（多头平仓）
        let close_long_kline = KlineInfo {
            close_price: 101.0,  // 改为 101.0，确保 >= 100.72
            high_price: 100.8,
            low_price: 100.2,
            open_price: 100.0,
            start_time: 0,
            close_time: 0,
            symbol: "TURBOUSDT".to_string(),
            interval: "1m".to_string(),
            first_trade_id: 0,
            last_trade_id: 0,
            base_volume: 0.0,
            trade_count: 0,
            is_closed: true,
            quote_volume: 0.0,
            taker_buy_base_volume: 0.0,
            taker_buy_quote_volume: 0.0,
            ignore: "".to_string(),
        };
        
        // 应该产生平仓信号
        if let Some(signal) = strategy.on_kline_update(&close_long_kline) {
            println!("平仓信号: {:?}", signal.side);
            assert_eq!(signal.side, Side::Sell); // 卖出平多
            assert_eq!(strategy.current_signal, 0); // 无持仓状态
            
            // 检查信号格式
            if let Signal::Market(market_signal) = signal.signal {
                assert_eq!(market_signal.is_closed, true); // 必须是平仓信号
                assert_eq!(signal.strategy, StrategyName::BOLLINGER);
                assert_eq!(signal.symbol, "TURBOUSDT");
                assert_eq!(signal.quantity, 10000.0);
            } else {
                panic!("应该是市场信号");
            }
        } else {
            panic!("应该产生平仓信号");
        }
    }

    #[test]
    fn test_bollinger_close_short_position() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        
        // 1. 先初始化策略
        for i in 0..20 {
            let price = 100.0 - (i as f64 * 0.1);
            let kline = KlineInfo {
                close_price: price,
                high_price: price + 0.05,
                low_price: price - 0.05,
                open_price: price + 0.02,
                start_time: 0,
                close_time: 0,
                symbol: "TURBOUSDT".to_string(),
                interval: "1m".to_string(),
                first_trade_id: 0,
                last_trade_id: 0,
                base_volume: 0.0,
                trade_count: 0,
                is_closed: true,
                quote_volume: 0.0,
                taker_buy_base_volume: 0.0,
                taker_buy_quote_volume: 0.0,
                ignore: "".to_string(),
            };
            strategy.on_kline_update(&kline);
        }
        
        // 2. 模拟做空信号（触及上轨）
        let short_kline = KlineInfo {
            close_price: 105.0,  // 触及上轨
            high_price: 105.5,
            low_price: 104.5,
            open_price: 104.8,
            start_time: 0,
            close_time: 0,
            symbol: "TURBOUSDT".to_string(),
            interval: "1m".to_string(),
            first_trade_id: 0,
            last_trade_id: 0,
            base_volume: 0.0,
            trade_count: 0,
            is_closed: true,
            quote_volume: 0.0,
            taker_buy_base_volume: 0.0,
            taker_buy_quote_volume: 0.0,
            ignore: "".to_string(),
        };
        
        // 应该产生做空信号
        if let Some(signal) = strategy.on_kline_update(&short_kline) {
            println!("做空信号: {:?}", signal.side);
            assert_eq!(signal.side, Side::Sell);
            assert_eq!(strategy.current_signal, 2); // 空头状态
        } else {
            panic!("应该产生做空信号");
        }
        
        // 3. 模拟价格穿过中线（空头平仓）
        let close_short_kline = KlineInfo {
            close_price: 99.5,  // 穿过中线
            high_price: 99.8,
            low_price: 99.2,
            open_price: 100.0,
            start_time: 0,
            close_time: 0,
            symbol: "TURBOUSDT".to_string(),
            interval: "1m".to_string(),
            first_trade_id: 0,
            last_trade_id: 0,
            base_volume: 0.0,
            trade_count: 0,
            is_closed: true,
            quote_volume: 0.0,
            taker_buy_base_volume: 0.0,
            taker_buy_quote_volume: 0.0,
            ignore: "".to_string(),
        };
        
        // 应该产生平仓信号
        if let Some(signal) = strategy.on_kline_update(&close_short_kline) {
            println!("平仓信号: {:?}", signal.side);
            assert_eq!(signal.side, Side::Buy); // 买入平空
            assert_eq!(strategy.current_signal, 0); // 无持仓状态
            
            // 检查信号格式
            if let Signal::Market(market_signal) = signal.signal {
                assert_eq!(market_signal.is_closed, true); // 必须是平仓信号
                assert_eq!(signal.strategy, StrategyName::BOLLINGER);
                assert_eq!(signal.symbol, "TURBOUSDT");
                assert_eq!(signal.quantity, 10000.0);
            } else {
                panic!("应该是市场信号");
            }
        } else {
            panic!("应该产生平仓信号");
        }
    }

    #[test]
    fn test_bollinger_no_signal_when_no_position() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        
        // 初始化策略
        for i in 0..20 {
            let price = 100.0 + (i as f64 * 0.1);
            let kline = KlineInfo {
                close_price: price,
                high_price: price + 0.05,
                low_price: price - 0.05,
                open_price: price - 0.02,
                start_time: 0,
                close_time: 0,
                symbol: "TURBOUSDT".to_string(),
                interval: "1m".to_string(),
                first_trade_id: 0,
                last_trade_id: 0,
                base_volume: 0.0,
                trade_count: 0,
                is_closed: true,
                quote_volume: 0.0,
                taker_buy_base_volume: 0.0,
                taker_buy_quote_volume: 0.0,
                ignore: "".to_string(),
            };
            strategy.on_kline_update(&kline);
        }
        
        // 没有持仓时，价格穿过中线不应该产生信号
        let cross_middle_kline = KlineInfo {
            close_price: 100.5,  // 穿过中线
            high_price: 100.8,
            low_price: 100.2,
            open_price: 100.0,
            start_time: 0,
            close_time: 0,
            symbol: "TURBOUSDT".to_string(),
            interval: "1m".to_string(),
            first_trade_id: 0,
            last_trade_id: 0,
            base_volume: 0.0,
            trade_count: 0,
            is_closed: true,
            quote_volume: 0.0,
            taker_buy_base_volume: 0.0,
            taker_buy_quote_volume: 0.0,
            ignore: "".to_string(),
        };
        
        // 应该没有信号
        let signal = strategy.on_kline_update(&cross_middle_kline);
        assert!(signal.is_none(), "没有持仓时不应该产生平仓信号");
        assert_eq!(strategy.current_signal, 0); // 保持无持仓状态
    }
}

