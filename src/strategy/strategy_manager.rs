use crate::common::ts::Strategy;
use crate::strategy::common::Signal;
use crate::strategy::macd::MacdStrategy;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use ta::{Close, High, Low, Open, Tbbav, Tbqav};

pub struct IdGenerator {
    base: u64,
    range_size: u64,
    counter: AtomicU64,
}

impl IdGenerator {
    pub fn new(range: (u64, u64)) -> Result<Self> {
        if range.0 >= range.1 {
            return Err(anyhow::anyhow!("range.0 must be less than range.1"));
        }
        Ok(Self {
            base: range.0,
            range_size: range.1 - range.0,
            counter: AtomicU64::new(0),
        })
    }

    // 🟢 最简版本 - 开销最小
    pub fn next_id(&self) -> u64 {
        let seq = self.counter.fetch_add(1, Ordering::Relaxed);
        self.base + (seq % self.range_size)
    }
}

// 使用枚举来支持不同的策略类型
#[derive(Clone)]
pub enum StrategyEnum {
    Macd(MacdStrategy),
    // 可以在这里添加更多策略类型
    // Rsi(RsiStrategy),
    // Bollinger(BollingerStrategy),
}

// 为 StrategyEnum 实现 Strategy trait
impl<T> Strategy<&T> for StrategyEnum
where
    T: High + Low + Close + Open + Tbbav + Tbqav,
{
    type Output = Signal;

    fn on_kline_update(&mut self, input: &T) -> Signal {
        match self {
            StrategyEnum::Macd(strategy) => strategy.on_kline_update(input),
            // StrategyEnum::Rsi(strategy) => strategy.on_kline_update(input),
            // StrategyEnum::Bollinger(strategy) => strategy.on_kline_update(input),
        }
    }
    fn name(&self) -> String {
        match self{
            StrategyEnum::Macd(_) => "MACD".to_string(),
            // StrategyEnum::Rsi(_) => "RSI".to_string(),
        }
    }
}

impl StrategyEnum {
    pub fn name(&self) -> String {
        match self {
            StrategyEnum::Macd(_) => "MACD".to_string(),
            // StrategyEnum::Rsi(_) => "RSI".to_string(),
        }
    }
}

impl<T> Strategy<Arc<T>> for StrategyEnum
where
    T: High + Low + Close + Open + Tbbav + Tbqav + Send + Sync + 'static,
{
    type Output = Signal;

    fn on_kline_update(&mut self, input: Arc<T>) -> Signal {
        match self {
            StrategyEnum::Macd(strategy) => strategy.on_kline_update(input),
            // StrategyEnum::Rsi(strategy) => strategy.on_kline_update(input),
            // StrategyEnum::Bollinger(strategy) => strategy.on_kline_update(input),
        }
    }

    fn name(&self) -> String {
        match self {
            StrategyEnum::Macd(_) => "MACD".to_string(),
            // StrategyEnum::Rsi(_) => "RSI".to_string(),
        }
    }
}

// 修改 StrategyManager 以使用枚举
pub struct StrategyManager<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + 'static> {
    strategies: Vec<StrategyEnum>,
    data_receiver: mpsc::Receiver<Arc<T>>,
    data_sender: mpsc::Sender<Signal>,
    next_id: Arc<AtomicU64>,
}

impl<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + 'static> StrategyManager<T> {
    pub fn new(data_receiver: mpsc::Receiver<Arc<T>>, data_sender: mpsc::Sender<Signal>) -> Self {
        Self {
            strategies: Vec::new(),
            data_receiver,
            data_sender,
            next_id: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn add_strategy(&mut self, strategy: StrategyEnum) {
        self.strategies.push(strategy);
    }

    pub fn run_single_strategy(&mut self, strategy_name: String) -> Result<()> {
        // 找到对应的策略
        let strategy_index = self.strategies.iter()
            .position(|s| s.name() == strategy_name)
            .ok_or_else(|| anyhow::anyhow!("Strategy '{}' not found", strategy_name))?;
        
        let mut strategy = self.strategies[strategy_index].clone();
        let data_sender = self.data_sender.clone();
        let _next_id = self.next_id.clone();

        // 在当前线程中运行策略（简化版本）
        while let Ok(data) = self.data_receiver.recv() {
            let signal = strategy.on_kline_update(data);
            
            // 直接发送原始信号，不进行检查
            if let Err(e) = data_sender.send(signal) {
                eprintln!("Failed to send signal: {}", e);
                break;
            }
        }
        
        Ok(())
    }

    pub fn run_all_strategies(&mut self) -> Result<()> {
        let data_sender = self.data_sender.clone();
        let mut strategies = self.strategies.clone();

        while let Ok(data) = self.data_receiver.recv() {
            for strategy in &mut strategies {
                let signal = strategy.on_kline_update(data.clone());
                
                if signal.is_actionable() {
                    if let Err(e) = data_sender.send(signal) {
                        eprintln!("Failed to send signal: {}", e);
                        break;
                    }
                }
            }
        }
        
        Ok(())
    }

    pub fn list_strategies(&self) -> Vec<String> {
        self.strategies.iter().map(|s| s.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::binance::websocket::KlineInfo;

    #[test]
    fn test_strategy_manager_creation() {
        let (data_tx, data_rx) = mpsc::channel();
        let (signal_tx, _signal_rx) = mpsc::channel();
        
        let manager: StrategyManager<KlineInfo> = StrategyManager::new(data_rx, signal_tx);
        assert_eq!(manager.strategies.len(), 0);
    }

    #[test]
    fn test_add_strategy() {
        let (data_tx, data_rx) = mpsc::channel();
        let (signal_tx, _signal_rx) = mpsc::channel();
        
        let mut manager: StrategyManager<KlineInfo> = StrategyManager::new(data_rx, signal_tx);
        let macd_strategy = MacdStrategy::new(20).unwrap();
        
        manager.add_strategy(StrategyEnum::Macd(macd_strategy));
        assert_eq!(manager.strategies.len(), 1);
        assert_eq!(manager.list_strategies(), vec!["MACD"]);
    }

    #[test]
    fn test_strategy_enum_name() {
        let macd_strategy = MacdStrategy::new(10).unwrap();
        let strategy_enum = StrategyEnum::Macd(macd_strategy);
        assert_eq!(strategy_enum.name(), "MACD");
    }

    #[test]
    fn test_id_generator() {
        let generator = IdGenerator::new((0, 5)).unwrap();
        assert_eq!(generator.next_id(), 0);
        assert_eq!(generator.next_id(), 1);
        assert_eq!(generator.next_id(), 2);
    }

    #[test]
    fn test_strategy_processing() {
        let (data_tx, data_rx) = mpsc::channel();
        let (signal_tx, signal_rx) = mpsc::channel();
        
        let mut manager: StrategyManager<KlineInfo> = StrategyManager::new(data_rx, signal_tx);
        let macd_strategy = MacdStrategy::new(5).unwrap();
        manager.add_strategy(StrategyEnum::Macd(macd_strategy));

        // 创建测试数据
        let test_kline = Arc::new(KlineInfo {
            start_time: 1638747660000,
            close_time: 1638747719999,
            symbol: "BTCUSDT".to_string(),
            interval: "1m".to_string(),
            first_trade_id: 100,
            last_trade_id: 200,
            open_price: 50000.0,
            close_price: 51000.0,
            high_price: 52000.0,
            low_price: 49000.0,
            base_volume: 1000.0,
            trade_count: 100,
            is_closed: true,
            quote_volume: 50000000.0,
            taker_buy_base_volume: 600.0,
            taker_buy_quote_volume: 30000000.0,
            ignore: "ignore".to_string(),
        });

        // 发送测试数据
        data_tx.send(test_kline).unwrap();
        drop(data_tx); // 关闭发送端

        // 运行策略（这会阻塞直到数据处理完成）
        let result = manager.run_single_strategy("MACD".to_string());
        assert!(result.is_ok());
    }
}

