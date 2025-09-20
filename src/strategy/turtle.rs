use ta::indicators::{Maximum, Minimum, AverageTrueRange};
use ta::{Next, Close, High, Low, Open};
use crate::common::enums::{Exchange, StrategyName};
use crate::models::{TradingSignal, Side};
use crate::common::ts::{Strategy, IsClosed};
use crate::common::consts::*;
use crate::common::utils::get_timestamp_ms;
use anyhow::Result;

#[derive(Clone)]
pub struct TurtleStrategy {
    pub max: Maximum,
    pub min: Minimum,
    pub atr: AverageTrueRange,
    pub count: usize,
    pub finish_init: bool,
    period: usize,
    last_price: f64,
    pub last_upper: f64,      // 最高点
    pub last_lower: f64,      // 最低点
    pub last_middle: f64,     // 中间值（可选，用于参考）
    pub last_atr: f64,
    pub current_signal: u8,   // 0: 已平仓, 1: 多头, 2: 空头
}

impl TurtleStrategy {
    pub fn new(period: usize) -> Result<Self> {
        Ok(Self {
            max: Maximum::new(period)?,
            min: Minimum::new(period)?,
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

    fn check_signals(&mut self, close_price: f64, high_price: f64, low_price: f64, upper_band: f64, lower_band: f64, atr_value: f64) -> Option<TradingSignal> {
        // 1. 检查是否需要平仓（如果有持仓且价格触及反向突破点）
        if self.current_signal != 0 {
            // 根据持仓方向判断是否需要平仓
            let should_close = match self.current_signal {
                1 => { // 多头持仓
                    // 价格跌破最低点，平多
                    close_price < lower_band
                }
                2 => { // 空头持仓
                    // 价格突破最高点，平空
                    close_price > upper_band
                }
                _ => false,
            };

            if should_close {
                let position_to_close = self.current_signal;  // 保存要平仓的位置
                self.current_signal = 0;  // 重置信号状态
                self.last_price = close_price;
                
                // 创建平仓信号
                let signal = TradingSignal::new_close_signal(
                    1,
                    TURBO_USDT_SYMBOL.to_string(),
                    position_to_close,  // 使用保存的位置
                    StrategyName::TURTLE,
                    1000.0,
                    Exchange::Binance,
                    close_price,
                );
                
                return Some(signal);
            }
        }

        // 2. 检查是否需要开新仓
        if self.current_signal == 0 {  // 只有在没有持仓时才开新仓
            if high_price > upper_band {
                // 突破最高点，做多
                let stop_price = close_price - (2.0 * atr_value); // 2倍ATR止损
                // 限制止损价格精度，避免币安精度错误
                let stop_price = (stop_price * 1000000.0).round() / 1000000.0;
                
                self.current_signal = 1;  // 设置为多头状态
                self.last_price = close_price;
                return Some(TradingSignal::new_market_signal(
                    1,
                    TURBO_USDT_SYMBOL.to_string(),
                    Side::Buy,
                    StrategyName::TURTLE,
                    10000.0,
                    Exchange::Binance,
                    get_timestamp_ms() as u32,
                    None,  // 止盈价格
                    Some(stop_price),  // 止损价格
                    close_price,
                ));
            } else if low_price < lower_band {
                // 跌破最低点，做空
                let stop_price = close_price + (2.0 * atr_value); // 2倍ATR止损
                // 限制止损价格精度，避免币安精度错误
                let stop_price = (stop_price * 1000000.0).round() / 1000000.0;
                
                self.current_signal = 2;  // 设置为空头状态
                self.last_price = close_price;
                
                return Some(TradingSignal::new_market_signal(
                    1,
                    TURBO_USDT_SYMBOL.to_string(),
                    Side::Sell,
                    StrategyName::TURTLE,
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

impl<T> Strategy<&T> for TurtleStrategy
where
    T: High + Low + Close + Open + IsClosed,
{
    type Output = Option<TradingSignal>;

    fn on_kline_update(&mut self, input: &T) -> Self::Output {
        if self.finish_init {
            let close_price = input.close();
            let high_price = input.high();
            let low_price = input.low();
            
            if input.is_closed() {
                // K线已收盘，使用next方法更新指标
                let max_value = self.max.next(input);
                let min_value = self.min.next(input);
                let atr_value = self.atr.next(input);
                
                // 更新最新的海龟通道值和ATR值
                self.last_upper = max_value;
                self.last_lower = min_value;
                self.last_middle = (max_value + min_value) / 2.0; // 计算中间值
                self.last_atr = atr_value;

                // 检查信号
                let signal = self.check_signals(close_price, high_price, low_price, max_value, min_value, atr_value);
                
                // 更新最新价格
                self.last_price = close_price;
                signal
            } else {
                // K线未收盘，使用上次的海龟通道值，ATR保持上次的值
                // 注意：Maximum和Minimum没有update方法，只能使用next方法
                // 在未收盘时，我们使用上次计算的值进行信号检测
                
                // 使用上次的ATR值进行信号检测
                let signal = self.check_signals(close_price, high_price, low_price, self.last_upper, self.last_lower, self.last_atr);
                
                // 更新最新价格
                self.last_price = close_price;
                signal
            }
        } else {
            // 初始化阶段
            if self.count < self.period {
                let max_value = self.max.next(input);
                let min_value = self.min.next(input);
                let atr_value = self.atr.next(input);
                self.count += 1;
                println!("海龟策略初始化中: {}/{}", self.count, self.period);
                
                if self.count == self.period {
                    self.finish_init = true;
                    self.last_price = input.close();
                    self.last_upper = max_value;
                    self.last_lower = min_value;
                    self.last_middle = (max_value + min_value) / 2.0;
                    self.last_atr = atr_value;
                    println!("海龟策略初始化完成，开始正常运行");
                    println!("初始通道: 上轨={:.4}, 下轨={:.4}, 中轨={:.4}, ATR={:.4}", 
                        max_value, min_value, self.last_middle, atr_value);
                }
            }
            None
        }
    }

    fn name(&self) -> String {
        "TURTLE".to_string()
    }
}

