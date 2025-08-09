use ta::indicators::{AverageTrueRange, BollingerBands};
use ta::{Next, Close, High, Low, Open};
use crate::common::enums::{Exchange, StrategyName};
use crate::common::signal::{TradingSignal, Side, Signal, MarketSignal};
use crate::common::ts::{Strategy, IsClosed};
use crate::common::consts::*;
use crate::common::utils::get_timestamp_ms;
use anyhow::Result;

#[derive(Clone)]
pub struct BollingerStrategy {
    pub bollinger: BollingerBands,
    pub atr: AverageTrueRange,
    pub count: usize,
    pub finish_init: bool,
    period: usize,
    last_price: f64,
    current_signal: u8,  // 0: 已平仓, 1: 多头, 2: 空头
}

impl BollingerStrategy {
    pub fn new(period: usize, std_dev: f64) -> Result<Self> {
        Ok(Self {
            bollinger: BollingerBands::new(period, std_dev)?,
            atr: AverageTrueRange::new(20)?,
            count: 0,
            finish_init: false,
            period,
            last_price: 0.0,
            current_signal: 0,
        })
    }

    // 辅助函数：生成平仓信号
    fn generate_close_signal(&mut self, price: f64) -> TradingSignal {
        // 根据当前持仓方向决定平仓方向
        let close_side = match self.current_signal {
            1 => Side::Sell,  // 持有多头，需要卖出平仓
            2 => Side::Buy,   // 持有空头，需要买入平仓
            _ => panic!("Unexpected state: generating close signal without position"),
        };

        // 生成平仓信号
        let signal = TradingSignal::new_market_signal(
            1,
            TURBO_USDT_SYMBOL.to_string(),
            close_side,
            StrategyName::BOLLINGER,
            10000.0,
            Exchange::Binance,
            get_timestamp_ms() as u32,
            None,
            None,
            price,
        );

        // 重置持仓状态
        self.current_signal = 0;
        signal
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
                // 计算布林带和ATR
                let bb_output = self.bollinger.next(input);
                let atr_value = self.atr.next(input);
                
                let upper_band = bb_output.upper;
                let lower_band = bb_output.lower;
                let middle_band = bb_output.average;

                // 1. 检查是否需要平仓（如果有持仓且价格触及中线）
                if self.current_signal != 0 {
                    // 根据持仓方向判断是否需要平仓
                    let should_close = match self.current_signal {
                        1 => self.last_price < middle_band && close_price >= middle_band, // 多头穿过中线
                        2 => self.last_price > middle_band && close_price <= middle_band, // 空头穿过中线
                        _ => false,
                    };

                    if should_close {
                        self.last_price = close_price;
                        return Some(self.generate_close_signal(close_price));
                    }
                }

                // 2. 检查是否需要开新仓
                if self.current_signal == 0 {  // 只有在没有持仓时才开新仓
                    if close_price >= upper_band {
                        // 触及上轨，做空
                        let stop_price = close_price + (2.0 * atr_value);
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
                            None,
                            Some(stop_price),
                            close_price,
                        ));
                    }
                }

                // 更新最新价格
                self.last_price = close_price;
                None
            } else {
                // 未收盘时只更新最新价格
                self.last_price = close_price;
                None
            }
        } else {
            // 初始化阶段
            if self.count < self.period {
                self.bollinger.next(input);
                self.atr.next(input);
                self.count += 1;
                println!("布林带初始化中: {}/{}", self.count, self.period);
                
                if self.count == self.period {
                    self.finish_init = true;
                    self.last_price = input.close();
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
}

