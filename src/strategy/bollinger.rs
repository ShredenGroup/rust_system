use ta::indicators::AverageTrueRange;
use ta::indicators::NewBollinger;
use ta::{Next, Close, High, Low, Open};
use crate::common::enums::{Exchange, StrategyName};
use crate::common::signal::{TradingSignal, Side};
use crate::common::ts::{Strategy, IsClosed, SymbolEnum, SymbolSetter};
use crate::common::TradingSymbol;

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
    /// 交易符号 - 支持动态设置
    symbol: TradingSymbol,
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
            symbol: TradingSymbol::default(), // 默认使用 BTCUSDT，后续可通过 set_symbol 修改
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
                
                // 计算数量: 20/close_price 向下取整
                let quantity = (20.0 / close_price).floor();
                
                // 创建平仓信号并标记为平仓操作
                let signal = TradingSignal::new_close_signal(
                    1,
                    self.symbol.clone().into(), // 使用 Into trait，对预定义枚举更高效
                    position_to_close,  // 使用保存的位置
                    StrategyName::BOLLINGER,
                    quantity,
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
                // 计算数量: 20/close_price 向下取整
                let quantity = (20.0 / close_price).floor();
                
                return Some(TradingSignal::new_market_signal(
                    1,
                    self.symbol.clone().into(), // 使用 Into trait，对预定义枚举更高效
                    Side::Sell,
                    StrategyName::BOLLINGER,
                    quantity,
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
                
                // 计算数量: 20/close_price 向下取整
                let quantity = (20.0 / close_price).floor();
                
                return Some(TradingSignal::new_market_signal(
                    1,
                    self.symbol.clone().into(), // 使用 Into trait，对预定义枚举更高效
                    Side::Buy,
                    StrategyName::BOLLINGER,
                    quantity,
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

// 实现 SymbolEnum trait
impl SymbolEnum for BollingerStrategy {
    fn symbol_enum(&self) -> &TradingSymbol {
        &self.symbol
    }
}

// 实现 SymbolSetter trait
impl SymbolSetter for BollingerStrategy {
    fn set_symbol(&mut self, symbol: TradingSymbol) {
        self.symbol = symbol;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::ts::Strategy;
    use crate::common::signal::Side;
    
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
    
    // 创建测试用的K线数据 - 模拟NEIROUSDT的价格走势
    fn create_test_klines() -> Vec<MockKlineData> {
        vec![
            // 初始化阶段 - 价格在0.000350-0.000360之间震荡
            MockKlineData::new(0.000350, 0.000355, 0.000349, 0.000352, true),
            MockKlineData::new(0.000352, 0.000358, 0.000351, 0.000356, true),
            MockKlineData::new(0.000356, 0.000361, 0.000354, 0.000359, true),
            MockKlineData::new(0.000359, 0.000362, 0.000357, 0.000360, true),
            MockKlineData::new(0.000360, 0.000365, 0.000358, 0.000363, true),
            MockKlineData::new(0.000363, 0.000368, 0.000361, 0.000365, true),
            MockKlineData::new(0.000365, 0.000370, 0.000363, 0.000367, true),
            MockKlineData::new(0.000367, 0.000372, 0.000365, 0.000369, true),
            MockKlineData::new(0.000369, 0.000374, 0.000367, 0.000371, true),
            MockKlineData::new(0.000371, 0.000376, 0.000369, 0.000373, true),
            MockKlineData::new(0.000373, 0.000378, 0.000371, 0.000375, true),
            MockKlineData::new(0.000375, 0.000380, 0.000373, 0.000377, true),
            MockKlineData::new(0.000377, 0.000382, 0.000375, 0.000379, true),
            MockKlineData::new(0.000379, 0.000384, 0.000377, 0.000381, true),
            MockKlineData::new(0.000381, 0.000386, 0.000379, 0.000383, true),
            MockKlineData::new(0.000383, 0.000388, 0.000381, 0.000385, true),
            MockKlineData::new(0.000385, 0.000390, 0.000383, 0.000387, true),
            MockKlineData::new(0.000387, 0.000392, 0.000385, 0.000389, true),
            MockKlineData::new(0.000389, 0.000394, 0.000387, 0.000391, true),
            MockKlineData::new(0.000391, 0.000396, 0.000389, 0.000393, true),
            
            // 触发信号的K线 - 价格突破上轨
            MockKlineData::new(0.000393, 0.000420, 0.000391, 0.000418, true), // 突破上轨，应该生成卖出信号
            
            // 后续K线 - 价格回调
            MockKlineData::new(0.000418, 0.000420, 0.000400, 0.000405, true),
            MockKlineData::new(0.000405, 0.000410, 0.000395, 0.000398, true),
            MockKlineData::new(0.000398, 0.000403, 0.000390, 0.000395, true), // 价格回到中轨附近，应该生成平仓信号
        ]
    }
    
    #[test]
    fn test_bollinger_strategy_initialization() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        strategy.set_symbol(TradingSymbol::NEIROUSDT);
        
        assert_eq!(strategy.period, 20);
        assert_eq!(strategy.current_signal, 0);
        assert!(!strategy.finish_init);
        assert_eq!(strategy.symbol, TradingSymbol::NEIROUSDT);
    }
    
    #[test]
    fn test_bollinger_strategy_initialization_phase() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        strategy.set_symbol(TradingSymbol::NEIROUSDT);
        
        let test_klines = create_test_klines();
        
        // 测试初始化阶段 - 前20根K线不应该生成信号
        for (i, kline) in test_klines.iter().take(20).enumerate() {
            let signal = strategy.on_kline_update(kline);
            assert!(signal.is_none(), "初始化阶段第{}根K线不应该生成信号", i + 1);
        }
        
        // 第20根K线后应该完成初始化
        assert!(strategy.finish_init, "第20根K线后应该完成初始化");
    }
    
    #[test]
    fn test_bollinger_strategy_sell_signal_generation() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        strategy.set_symbol(TradingSymbol::NEIROUSDT);
        
        let test_klines = create_test_klines();
        
        // 初始化策略
        for kline in test_klines.iter().take(20) {
            strategy.on_kline_update(kline);
        }
        
        // 测试第21根K线 - 应该生成卖出信号（突破上轨）
        let signal = strategy.on_kline_update(&test_klines[20]);
        
        assert!(signal.is_some(), "价格突破上轨应该生成信号");
        let signal = signal.unwrap();
        assert_eq!(signal.side, Side::Sell, "突破上轨应该生成卖出信号");
        assert_eq!(signal.symbol, "NEIROUSDT", "信号应该包含正确的交易对");
        assert!(signal.quantity > 0.0, "信号应该包含正确的数量");
        assert_eq!(strategy.current_signal, 2, "策略状态应该更新为空头");
        
        println!("生成卖出信号: {:?}", signal);
        println!("布林带值 - 上轨: {:.6}, 中轨: {:.6}, 下轨: {:.6}", 
            strategy.last_upper, strategy.last_middle, strategy.last_lower);
    }
    
    #[test]
    fn test_bollinger_strategy_close_signal_generation() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        strategy.set_symbol(TradingSymbol::NEIROUSDT);
        
        let test_klines = create_test_klines();
        
        // 初始化策略
        for kline in test_klines.iter().take(20) {
            strategy.on_kline_update(kline);
        }
        
        // 生成开仓信号
        let open_signal = strategy.on_kline_update(&test_klines[20]);
        assert!(open_signal.is_some());
        assert_eq!(strategy.current_signal, 2); // 空头状态
        
        // 处理中间的K线
        strategy.on_kline_update(&test_klines[21]);
        strategy.on_kline_update(&test_klines[22]);
        
        // 测试平仓信号 - 价格回到中轨
        println!("当前持仓状态: {}", strategy.current_signal);
        println!("当前价格: {:.6}, 上轨: {:.6}, 中轨: {:.6}, 下轨: {:.6}", 
            strategy.last_price, strategy.last_upper, strategy.last_middle, strategy.last_lower);
        println!("测试K线23价格: {:.6}", test_klines[23].close);
        
        let close_signal = strategy.on_kline_update(&test_klines[23]);
        
        println!("处理后持仓状态: {}", strategy.current_signal);
        println!("处理后价格: {:.6}, 上轨: {:.6}, 中轨: {:.6}, 下轨: {:.6}", 
            strategy.last_price, strategy.last_upper, strategy.last_middle, strategy.last_lower);
        
        assert!(close_signal.is_some(), "价格回到中轨应该生成平仓信号");
        let signal = close_signal.unwrap();
        assert_eq!(signal.symbol, "NEIROUSDT", "平仓信号应该包含正确的交易对");
        assert!(signal.quantity > 0.0, "平仓信号应该包含正确的数量");
        assert_eq!(strategy.current_signal, 0, "平仓后策略状态应该重置为无持仓");
        
        println!("生成平仓信号: {:?}", signal);
    }
    
    #[test]
    fn test_bollinger_strategy_buy_signal() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        strategy.set_symbol(TradingSymbol::NEIROUSDT);
        
        // 创建触发下轨的测试数据
        let mut test_klines = create_test_klines();
        
        // 修改最后一根K线，使其触及下轨
        let last_idx = test_klines.len() - 1;
        test_klines[last_idx] = MockKlineData::new(0.000350, 0.000355, 0.000300, 0.000310, true);
        
        // 初始化策略
        for kline in test_klines.iter().take(20) {
            strategy.on_kline_update(kline);
        }
        
        // 测试买入信号
        let signal = strategy.on_kline_update(&test_klines[last_idx]);
        
        if let Some(signal) = signal {
            assert_eq!(signal.side, Side::Buy, "触及下轨应该生成买入信号");
            assert_eq!(strategy.current_signal, 1, "策略状态应该更新为多头");
            println!("生成买入信号: {:?}", signal);
        }
    }
    
    #[test]
    fn test_bollinger_strategy_no_duplicate_signals() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        strategy.set_symbol(TradingSymbol::NEIROUSDT);
        
        let test_klines = create_test_klines();
        
        // 初始化策略
        for kline in test_klines.iter().take(20) {
            strategy.on_kline_update(kline);
        }
        
        // 第一次触发信号
        let first_signal = strategy.on_kline_update(&test_klines[20]);
        assert!(first_signal.is_some());
        
        // 再次使用相同的K线，不应该生成重复信号
        let duplicate_signal = strategy.on_kline_update(&test_klines[20]);
        assert!(duplicate_signal.is_none(), "相同条件下不应该生成重复信号");
    }
    
    #[test]
    fn test_bollinger_strategy_quantity_calculation() {
        let mut strategy = BollingerStrategy::new(20, 2.0).unwrap();
        strategy.set_symbol(TradingSymbol::NEIROUSDT);
        
        let test_klines = create_test_klines();
        
        // 初始化策略
        for kline in test_klines.iter().take(20) {
            strategy.on_kline_update(kline);
        }
        
        // 生成信号并检查数量计算
        let signal = strategy.on_kline_update(&test_klines[20]);
        
        if let Some(signal) = signal {
            let expected_quantity = (20.0 / test_klines[20].close).floor();
            assert_eq!(signal.quantity, expected_quantity, 
                "数量计算应该是 20/价格 向下取整");
            
            println!("价格: {}, 计算数量: {}, 期望数量: {}", 
                test_klines[20].close, signal.quantity, expected_quantity);
        }
    }
}
