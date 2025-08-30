use ta::indicators::AverageTrueRange;
use ta::indicators::NewBollinger;
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
                    1000.0,
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
                let stop_price = close_price + (1.0 * atr_value);
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
                let stop_price = close_price - (1.0 * atr_value);
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
