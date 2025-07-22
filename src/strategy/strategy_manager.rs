use crate::strategy::common::Signal;
use crate::common::ts::Strategy;
use crate::strategy::macd::MacdStrategy;
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use ta::{Close, High, Low, Open, Tbbav, Tbqav};
use std::thread;
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
}

// 修改 StrategyManager 以使用枚举
pub struct StrategyManager<T: Close + High + Open + Low + Tbbav + Tbqav + Send + Sync + 'static> {
    strategies: Vec<StrategyEnum>,
    data_receiver: mpsc::Receiver<Arc<T>>,
    data_sender: mpsc::Sender<Signal>,
    next_id: Arc<AtomicU64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multithread() {
        let signal_id_generator = Arc::new(IdGenerator::new((0, 2000)).unwrap());

        // 克隆 Arc 用于第一个线程
        let signal_id_generator_clone1 = Arc::clone(&signal_id_generator);
        let handle1 = thread::spawn(move || {
            for _i in 0..1000 {
                let _new_id = signal_id_generator_clone1.next_id();
            }
        });

        // 克隆 Arc 用于第二个线程
        let signal_id_generator_clone2 = Arc::clone(&signal_id_generator);
        let handle2 = thread::spawn(move || {
            for _i in 0..1000 {
                let _new_id = signal_id_generator_clone2.next_id();
            }
        });

        // 等待两个线程完成
        handle1.join().unwrap();
        handle2.join().unwrap();

        // 验证ID生成器仍然工作
        let final_id = signal_id_generator.next_id();
        assert!(final_id >= 0 && final_id < 2000);
    }

    #[test]
    fn test_single_thread() {
        let generator = IdGenerator::new((1000, 1003)).unwrap();

        // 测试连续生成ID
        assert_eq!(generator.next_id(), 1000); // 起始值
        assert_eq!(generator.next_id(), 1001); // +1
        assert_eq!(generator.next_id(), 1002); // +1
        assert_eq!(generator.next_id(), 1000); // 循环回到起始值
        assert_eq!(generator.next_id(), 1001); // 再次循环
    }

    #[test]
    fn test_different_ranges() {
        let generator1 = IdGenerator::new((0, 5)).unwrap();
        let generator2 = IdGenerator::new((10000, 10005)).unwrap();

        // 测试第一个生成器
        assert_eq!(generator1.next_id(), 0);
        assert_eq!(generator1.next_id(), 1);
        assert_eq!(generator1.next_id(), 2);
        assert_eq!(generator1.next_id(), 3);
        assert_eq!(generator1.next_id(), 4);
        assert_eq!(generator1.next_id(), 0); // 循环

        // 测试第二个生成器
        assert_eq!(generator2.next_id(), 10000);
        assert_eq!(generator2.next_id(), 10001);
        assert_eq!(generator2.next_id(), 10002);
        assert_eq!(generator2.next_id(), 10003);
        assert_eq!(generator2.next_id(), 10004);
        assert_eq!(generator2.next_id(), 10000); // 循环
    }

    #[test]
    fn test_invalid_range() {
        // 测试无效范围
        assert!(IdGenerator::new((10, 5)).is_err());
        assert!(IdGenerator::new((5, 5)).is_err());

        // 测试有效范围
        assert!(IdGenerator::new((0, 1)).is_ok());
        assert!(IdGenerator::new((0, 100)).is_ok());
    }

    #[test]
    fn test_range_cycling() {
        let generator = IdGenerator::new((100, 103)).unwrap();

        // 生成多个周期的ID
        let mut ids = Vec::new();
        for _ in 0..10 {
            ids.push(generator.next_id());
        }

        // 验证循环模式
        assert_eq!(ids, vec![100, 101, 102, 100, 101, 102, 100, 101, 102, 100]);
    }
}
