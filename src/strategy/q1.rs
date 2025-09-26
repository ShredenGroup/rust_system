use ta::indicators::{Maximum, Minimum, AverageTrueRange, ExponentialMovingAverage};
use ta::{Next, Close, High, Low, Open};
use crate::common::enums::{Exchange, StrategyName};
use crate::models::{TradingSignal, Side, TradingSymbol};
use crate::common::ts::{Strategy, IsClosed, SymbolEnum, SymbolSetter};
use crate::common::utils::{get_timestamp_ms, align_price_precision};
use crate::signal_log;
use anyhow::Result;

#[derive(Clone)]
pub struct Q1Strategy {
    // 突破周期的最高最低价指标
    pub max_break: Maximum,
    pub min_break: Minimum,
    // 前一根K线的最高最低价
    pub prev_high: f64,
    pub prev_low: f64,
    // EMA指标
    pub ema: ExponentialMovingAverage,
    // 止盈周期的最高最低价指标
    pub max_profit: Maximum,
    pub min_profit: Minimum,
    // ATR指标（用于止损）
    pub atr: AverageTrueRange,
    // 计数器和初始化标志
    pub count: usize,
    pub finish_init: bool,
    // 当前持仓状态 (0: 无持仓, 1: 多头, 2: 空头)
    pub current_signal: u8,
    // 缓存最新价格和指标值
    pub last_price: f64,
    // 最近一次开仓时确定的止损价（多单: 价格-ATR*k，空单: 价格+ATR*k）
    pub last_stop_price: Option<f64>,
    pub last_ema: f64,
    pub last_atr: f64,
    pub last_upper_break: f64,
    pub last_lower_break: f64,
    pub last_max_profit: f64,
    pub last_min_profit: f64,
    // ATR倍数
    pub atr_multiplier: f64,
    // 交易符号 - 支持动态设置
    symbol: TradingSymbol,
}

impl Q1Strategy {
    pub fn new(
        break_period: usize,     // 突破周期（默认35）
        ema_period: usize,       // EMA周期（默认240）
        profit_period: usize,    // 止盈周期（默认10）
        atr_period: usize,       // ATR周期（默认20）
        atr_multiplier: f64,     // ATR倍数（默认3.0）
        symbol: Option<TradingSymbol>, // 交易对（可选）
    ) -> Result<Self> {
        Ok(Self {
            max_break: Maximum::new(break_period)?,
            min_break: Minimum::new(break_period)?,
            prev_high: 0.0,
            prev_low: 0.0,
            ema: ExponentialMovingAverage::new(ema_period)?,
            max_profit: Maximum::new(profit_period)?,
            min_profit: Minimum::new(profit_period)?,
            atr: AverageTrueRange::new(atr_period)?,
            count: 0,
            finish_init: false,
            current_signal: 0,
            last_price: 0.0,
            last_stop_price: None,
            last_ema: 0.0,
            last_atr: 0.0,
            last_upper_break: 0.0,
            last_lower_break: 0.0,
            last_max_profit: 0.0,
            last_min_profit: 0.0,
            atr_multiplier: atr_multiplier,
            symbol: symbol.unwrap_or_default(), // 如果没有提供symbol，使用默认值
        })
    }

    /// 使用默认参数创建策略实例
    pub fn default() -> Result<Self> {
        Self::new(
            35,    // break_period
            240,   // ema_period
            10,    // profit_period
            20,    // atr_period
            3.0,   // atr_multiplier
            None,  // symbol
        )
    }

    fn check_signals(&mut self, close_price: f64, high_price: f64, low_price: f64, 
                    max_break: f64, min_break: f64, ema_value: f64, max_profit: f64, min_profit: f64, 
                    atr_value: f64) -> Option<TradingSignal> {
        // 1. 检查是否需要平仓
        if self.current_signal != 0 {
            let should_close = match self.current_signal {
                1 => { // 多头持仓，当价格跌破止盈周期低点时平仓
                    close_price < min_profit
                }
                2 => { // 空头持仓，当价格突破止盈周期高点时平仓
                    close_price > max_profit
                }
                _ => false,
            };

            if should_close {
                // 风控：若价格已触发止损（多单<=止损；空单>=止损），则不发送平仓信号
                if let Some(stop) = self.last_stop_price {
                    let violated = match self.current_signal {
                        1 => close_price <= stop, // 多单
                        2 => close_price >= stop, // 空单
                        _ => false,
                    };
                    if violated {
                        return None;
                    }
                }
                let position_to_close = self.current_signal;
                self.current_signal = 0;
                self.last_price = close_price;
                
                // 计算数量: 50/close_price 向下取整，最小0.001
                let quantity = (50.0 / close_price).floor().max(0.001);
                
                return Some(TradingSignal::new_close_signal(
                    1,
                    self.symbol.clone().into(),
                    position_to_close,
                    StrategyName::TURTLE,
                    quantity,
                    Exchange::Binance,
                    close_price,
                ));
            }
        }

        // 2. 检查是否需要开新仓
        if self.current_signal == 0 {
            // 开多条件：
            // 1. 当前K线突破35根K线的高点
            // 2. 前一根K线没有突破35根K线的高点
            // 3. 价格在240 EMA上方
            if high_price > max_break && self.prev_high < max_break && close_price > ema_value {
                let raw_stop_price = close_price - (self.atr_multiplier * atr_value); // ATR止损
                let stop_price = align_price_precision(close_price, raw_stop_price); // 与市场价格对齐精度
                
                signal_log!(info, "🎯 Q1策略发出开多信号: 交易对={}, 当前价格={:.8}, 原始止损价={:.8}, 对齐后止损价={:.8}", 
                    self.symbol.as_str(), close_price, raw_stop_price, stop_price);
                
                self.current_signal = 1;
                self.last_price = close_price;
                self.last_stop_price = Some(stop_price);
                // 计算数量: 50/close_price 向下取整，最小0.001
                let quantity = (50.0 / close_price).floor().max(0.001);
                
                signal_log!(info, "📊 开多信号详情: 数量={:.8}, 止损价={:.8}, 价格精度对齐完成", quantity, stop_price);
                
                return Some(TradingSignal::new_market_signal(
                    1,
                    self.symbol.clone().into(),
                    Side::Buy,
                    StrategyName::TURTLE,
                    quantity,
                    Exchange::Binance,
                    get_timestamp_ms() as u32,
                    None,
                    Some(stop_price),
                    close_price,
                ));
            } 
            // 开空条件：
            // 1. 当前K线突破35根K线的低点
            // 2. 前一根K线没有突破35根K线的低点
            // 3. 价格在240 EMA下方
            else if low_price < min_break && self.prev_low >= min_break && close_price < ema_value {
                let raw_stop_price = close_price + (self.atr_multiplier * atr_value); // ATR止损
                let stop_price = align_price_precision(close_price, raw_stop_price); // 与市场价格对齐精度
                
                signal_log!(info, "🎯 Q1策略发出开空信号: 交易对={}, 当前价格={:.8}, 原始止损价={:.8}, 对齐后止损价={:.8}", 
                    self.symbol.as_str(), close_price, raw_stop_price, stop_price);
                
                self.current_signal = 2;
                self.last_price = close_price;
                self.last_stop_price = Some(stop_price);
                // 计算数量: 20/close_price 向下取整，最小0.001
                let quantity = (50.0 / close_price).floor().max(0.001);
                
                signal_log!(info, "📊 开空信号详情: 数量={:.8}, 止损价={:.8}, 价格精度对齐完成", quantity, stop_price);
                
                return Some(TradingSignal::new_market_signal(
                    1,
                    self.symbol.clone().into(),
                    Side::Sell,
                    StrategyName::TURTLE,
                    quantity,
                    Exchange::Binance,
                    get_timestamp_ms() as u32,
                    None,
                    Some(stop_price),
                    close_price,
                ));
            }
        }

        None
    }
}

impl<T> Strategy<&T> for Q1Strategy
where
    T: High + Low + Close + Open + IsClosed,
{
    type Output = Option<TradingSignal>;

    fn on_kline_update(&mut self, input: &T) -> Self::Output {
        let close_price = input.close();
        let high_price = input.high();
        let low_price = input.low();

        if self.finish_init {
            if input.is_closed() {
                // 更新前一根K线的高低点
                self.prev_high = high_price;
                self.prev_low = low_price;
                
                // 更新所有指标
                let max_break = self.max_break.next(input);
                let min_break = self.min_break.next(input);
                let ema_value = self.ema.next(input);
                let max_profit = self.max_profit.next(input);
                let min_profit = self.min_profit.next(input);
                let atr_value = self.atr.next(input);

                // 缓存最新的指标值
                self.last_ema = ema_value;
                self.last_atr = atr_value;
                self.last_upper_break = max_break;
                self.last_lower_break = min_break;
                self.last_max_profit = max_profit;
                self.last_min_profit = min_profit;

                // 检查信号
                let signal = self.check_signals(
                    close_price, high_price, low_price,
                    max_break, min_break, ema_value, max_profit, min_profit, atr_value
                );

                self.last_price = close_price;
                signal
            } else {
                // K线未收盘，使用缓存的指标值进行信号检测
                // 在未收盘时使用上一次计算的指标值
                // 在未收盘时使用上一次计算的指标值
                let signal = self.check_signals(
                    close_price, high_price, low_price,
                    self.last_upper_break, self.last_lower_break,
                    self.last_ema, self.last_max_profit, self.last_min_profit,
                    self.last_atr
                );

                self.last_price = close_price;
                signal
            }
        } else {
            // 初始化阶段
            if self.count < 240 { // 使用最长的指标周期作为初始化周期
                self.max_break.next(input);
                self.min_break.next(input);
                self.ema.next(input);
                self.max_profit.next(input);
                self.min_profit.next(input);
                let atr_value = self.atr.next(input);
                self.count += 1;
                println!("Q1策略初始化中: {}/240", self.count);
                
                if self.count == 240 {
                    self.finish_init = true;
                    self.last_price = close_price;
                    self.prev_high = high_price;
                    self.prev_low = low_price;
                    let ema_value = self.ema.next(input);
                    let max_break = self.max_break.next(input);
                    let min_break = self.min_break.next(input);
                    let max_profit = self.max_profit.next(input);
                    let min_profit = self.min_profit.next(input);
                    
                    self.last_ema = ema_value;
                    self.last_atr = atr_value;
                    self.last_upper_break = max_break;
                    self.last_lower_break = min_break;
                    self.last_max_profit = max_profit;
                    self.last_min_profit = min_profit;
                    println!("Q1策略初始化完成，开始正常运行");
                }
            }
            None
        }
    }

    fn name(&self) -> String {
        "Q1".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // 模拟K线数据结构
    #[derive(Clone)]
    struct MockKlineData {
        open: f64,
        high: f64,
        low: f64,
        close: f64,
        is_closed: bool,
    }
    
    impl MockKlineData {
        fn new(open: f64, high: f64, low: f64, close: f64, is_closed: bool) -> Self {
            Self { open, high, low, close, is_closed }
        }
    }
    
    impl Open for MockKlineData {
        fn open(&self) -> f64 { self.open }
    }
    
    impl High for MockKlineData {
        fn high(&self) -> f64 { self.high }
    }
    
    impl Low for MockKlineData {
        fn low(&self) -> f64 { self.low }
    }
    
    impl Close for MockKlineData {
        fn close(&self) -> f64 { self.close }
    }
    
    impl IsClosed for MockKlineData {
        fn is_closed(&self) -> bool { self.is_closed }
    }
    
    // 创建测试用的K线数据
    fn create_test_klines() -> Vec<MockKlineData> {
        // 创建一个上升趋势的数据序列，每个数据点略高于前一个
        let mut klines = Vec::with_capacity(240);
        let base_price = 1000.0;
        
        for i in 0..240 {
            let trend = 0.1; // 每个周期的趋势
            let volatility = 0.02; // 波动率
            
            let open = base_price * (1.0 + trend * (i as f64 / 240.0));
            let high = open * (1.0 + volatility);
            let low = open * (1.0 - volatility);
            let close = (high + low) / 2.0;
            
            klines.push(MockKlineData::new(open, high, low, close, true));
        }
        
        klines
    }
    
    #[test]
    fn test_q1_strategy_initialization() {
        let mut strategy = Q1Strategy::new(
            35,    // break_period
            240,   // ema_period
            10,    // profit_period
            20,    // atr_period
            3.0,   // atr_multiplier
            None,  // symbol
        ).unwrap();
        
        assert_eq!(strategy.current_signal, 0);
        assert!(!strategy.finish_init);
        
        // 测试默认symbol
        assert_eq!(strategy.symbol_enum(), &TradingSymbol::default());
        
        // 测试设置symbol
        strategy.set_symbol(TradingSymbol::BTCUSDT);
        assert_eq!(strategy.symbol_enum(), &TradingSymbol::BTCUSDT);
    }
    
    #[test]
    fn test_q1_strategy_initialization_phase() {
        let mut strategy = Q1Strategy::default().unwrap();
        let test_klines = create_test_klines();
        
        // 测试初始化阶段 - 前240根K线不应该生成信号
        for (i, kline) in test_klines.iter().enumerate() {
            let signal = strategy.on_kline_update(kline);
            assert!(signal.is_none(), "初始化阶段第{}根K线不应该生成信号", i + 1);
        }
        
        // 第240根K线后应该完成初始化
        assert!(strategy.finish_init, "第240根K线后应该完成初始化");
    }
    
    #[test]
    fn test_q1_strategy_buy_signal() {
        let mut strategy = Q1Strategy::default().unwrap();
        let test_klines = create_test_klines();
        
        // 初始化策略
        for kline in test_klines.iter().take(240) {
            strategy.on_kline_update(kline);
        }
        
        // 获取最后一个初始化K线的价格作为基准
        let last_price = test_klines.last().unwrap().close;
        
        // 创建一个突破性的K线
        // 前一根K线没有突破，当前K线突破35周期高点，且价格在EMA上方
        let breakthrough_kline = MockKlineData::new(
            last_price * 1.02,             // open: 高于前收盘
            last_price * 1.05,             // high: 显著突破前高
            last_price * 1.015,            // low: 保持在较高水平
            last_price * 1.045,            // close: 收在高位
            true
        );
        
        let signal = strategy.on_kline_update(&breakthrough_kline);
        
        assert!(signal.is_some(), "突破高点应该生成信号");
        if let Some(signal) = signal {
            assert_eq!(signal.side, Side::Buy, "应该生成买入信号");
            assert_eq!(strategy.current_signal, 1, "策略状态应该更新为多头");
        }
    }
    
    #[test]
    fn test_q1_strategy_sell_signal() {
        let mut strategy = Q1Strategy::default().unwrap();
        let test_klines = create_test_klines();
        
        // 初始化策略
        for kline in test_klines.iter().take(240) {
            strategy.on_kline_update(kline);
        }
        
        // 获取最后一个初始化K线的价格作为基准
        let last_price = test_klines.last().unwrap().close;
        
        // 创建一个突破性的K线
        // 前一根K线没有突破，当前K线突破35周期低点，且价格在EMA下方
        let breakthrough_kline = MockKlineData::new(
            last_price * 0.98,             // open: 低于前收盘
            last_price * 0.985,            // high: 保持在低位
            last_price * 0.95,             // low: 显著突破前低
            last_price * 0.955,            // close: 收在低位
            true
        );
        
        let signal = strategy.on_kline_update(&breakthrough_kline);
        
        assert!(signal.is_some(), "突破低点应该生成信号");
        if let Some(signal) = signal {
            assert_eq!(signal.side, Side::Sell, "应该生成卖出信号");
            assert_eq!(strategy.current_signal, 2, "策略状态应该更新为空头");
        }
    }
    
    #[test]
    fn test_q1_strategy_profit_taking() {
        let mut strategy = Q1Strategy::default().unwrap();
        let test_klines = create_test_klines();
        
        // 初始化策略并开多仓
        for kline in test_klines.iter().take(240) {
            strategy.on_kline_update(kline);
        }
        
        // 获取最后一个初始化K线的价格作为基准
        let last_price = test_klines.last().unwrap().close;
        
        // 创建开仓信号
        let open_kline = MockKlineData::new(
            last_price * 1.02,             // open: 高于前收盘
            last_price * 1.05,             // high: 显著突破前高
            last_price * 1.015,            // low: 保持在较高水平
            last_price * 1.045,            // close: 收在高位
            true
        );
        let open_signal = strategy.on_kline_update(&open_kline);
        assert!(open_signal.is_some(), "应该生成开仓信号");
        assert_eq!(strategy.current_signal, 1, "应该开多仓"); 
        
        // 创建一个触发止盈的K线（跌破10周期低点）
        let profit_taking_kline = MockKlineData::new(
            last_price * 0.98,             // open: 开始下跌
            last_price * 0.985,            // high: 维持在低位
            last_price * 0.95,             // low: 跌破10周期低点
            last_price * 0.955,            // close: 收在低位
            true
        );
        
        let close_signal = strategy.on_kline_update(&profit_taking_kline);
        
        assert!(close_signal.is_some(), "跌破止盈点应该生成平仓信号");
        assert_eq!(strategy.current_signal, 0, "平仓后策略状态应该重置为无持仓");
    }
}

// 实现 SymbolEnum trait
impl SymbolEnum for Q1Strategy {
    fn symbol_enum(&self) -> &TradingSymbol {
        &self.symbol
    }
}

// 实现 SymbolSetter trait
impl SymbolSetter for Q1Strategy {
    fn set_symbol(&mut self, symbol: TradingSymbol) {
        self.symbol = symbol;
    }
}
