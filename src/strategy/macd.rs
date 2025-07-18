use crate::common::ts::Strategy;
use crate::strategy::common::{Signal, SignalType};
use crate::{
    common::config::ws_config::{KlineConfig, WebSocketBaseConfig},
    exchange_api::binance::ws_manager::{WebSocketMessage, create_websocket_manager},
};
use anyhow::Result;
use ta::indicators::hbfc_one::HbfcOne;
use ta::indicators::{SimpleMovingAverage, hbfc_one};
use ta::{Close, High, Low, Next, Open, Tbbav, Tbqav};
use std::sync::Arc;
use rayon::prelude::*;
#[derive(Clone)]
pub struct MacdStrategy {
    pub ema: SimpleMovingAverage,
    pub hbfc: hbfc_one::HbfcOne,
}

impl MacdStrategy {
    pub fn new(period: usize) -> Result<Self> {
        let ema = SimpleMovingAverage::new(period)?;
        let hbfc = HbfcOne::new();
        Ok(Self { ema, hbfc })
    }
}

// 为引用类型实现 Strategy trait
impl<T> Strategy<&T> for MacdStrategy
where
    T: High + Low + Close + Open + Tbbav + Tbqav,
{
    type Output = Signal;
    fn on_kline_update(&mut self, input: &T) -> Signal {
        // 顺序计算，简单高效
        let hbfc_val = self.hbfc.next(input);
        let ema_val = self.ema.next(input);
        
        println!("New hbfc_val{:?}", hbfc_val);
        println!("New ema_val{:?}", ema_val);
        
        // 后续逻辑...
        if hbfc_val.is_some() && hbfc_val.unwrap() > 0.5 {
            Signal::buy("BTCUSDT".to_string(), input.close(), 0.1)
        } else if hbfc_val.is_some() && hbfc_val.unwrap() < -0.5 {
            Signal::sell("BTCUSDT".to_string(), input.close(), 0.1)
        } else {
            Signal {
                signal_type: None,
                symbol: "BTCUSDT".to_string(),
                price: input.close(),
                quantity: 0.0,
                timestamp: chrono::Utc::now().timestamp(),
            }
        }
    }
}

// 为 Arc<T> 类型实现 Strategy trait - 泛型实现
impl<T> Strategy<Arc<T>> for MacdStrategy
where
    T: High + Low + Close + Open + Tbbav + Tbqav+Sync+Send,
{
    type Output = Signal;
    fn on_kline_update(&mut self, input: Arc<T>) -> Signal {
        let hbfc_val = self.hbfc.next(input.as_ref());
        let _ema = self.ema.next(input.as_ref());
        // 示例逻辑：根据指标值决定信号
        if hbfc_val.is_some() && hbfc_val.unwrap() > 0.5 {
            Signal::buy("BTCUSDT".to_string(), input.close(), 0.1)
        } else if hbfc_val.is_some() && hbfc_val.unwrap() < -0.5 {
            Signal::sell("BTCUSDT".to_string(), input.close(), 0.1)
        } else {
            // 创建带有正确 symbol 信息的 hold 信号
            Signal {
                signal_type: None,
                symbol: "BTCUSDT".to_string(),
                price: input.close(),
                quantity: 0.0,
                timestamp: chrono::Utc::now().timestamp(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dto::binance::websocket::{KlineData, KlineInfo};
    use std::sync::Arc;

    #[test]
    fn test_macd_strategy_creation() {
        let strategy = MacdStrategy::new(20);
        assert!(strategy.is_ok());
    }

    #[test]
    fn test_strategy_with_sample_data() {
        let mut strategy = MacdStrategy::new(10).unwrap();
        
        // 创建测试用的 KlineInfo 数据
        let kline_info = KlineInfo {
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
        };

        // 测试 Strategy trait 实现
        let signal = strategy.on_kline_update(&kline_info);
        println!("Generated signal: {:?}", signal);
        
        // 验证信号的基本属性
        assert_eq!(signal.symbol, "BTCUSDT");
        assert_eq!(signal.price, 51000.0);
        assert!(signal.timestamp > 0);
    }

    #[test]
    fn test_strategy_with_arc_kline_info() {
        let mut strategy = MacdStrategy::new(10).unwrap();
        
        // 创建测试用的 KlineInfo 数据
        let kline_info = KlineInfo {
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
        };

        // 包装在 Arc 中
        let arc_kline = Arc::new(kline_info);

        // 测试 Strategy<Arc<KlineInfo>> trait 实现
        let signal = strategy.on_kline_update(arc_kline);
        println!("Generated signal from Arc<KlineInfo>: {:?}", signal);
        
        // 验证信号的基本属性
        assert_eq!(signal.symbol, "BTCUSDT");
        assert_eq!(signal.price, 51000.0);
        assert!(signal.timestamp > 0);
    }

    #[test]
    fn test_strategy_with_arc_websocket_data() {
        let mut strategy = MacdStrategy::new(5).unwrap();
        
        // 创建测试用的 WebSocket KlineData
        let kline_data = KlineData {
            event_type: "kline".to_string(),
            event_time: 1638747660000,
            symbol: "BTCUSDT".to_string(),
            kline: KlineInfo {
                start_time: 1638747660000,
                close_time: 1638747719999,
                symbol: "BTCUSDT".to_string(),
                interval: "1m".to_string(),
                first_trade_id: 100,
                last_trade_id: 200,
                open_price: 45000.0,
                close_price: 46000.0,
                high_price: 47000.0,
                low_price: 44000.0,
                base_volume: 500.0,
                trade_count: 50,
                is_closed: true,
                quote_volume: 23000000.0,
                taker_buy_base_volume: 300.0,
                taker_buy_quote_volume: 13800000.0,
                ignore: "ignore".to_string(),
            },
        };

        // 注意：WebSocketKlineData 本身不实现 ta 的 traits，我们需要使用 kline 字段
        // 所以我们创建一个 Arc<KlineInfo> 来测试
        let arc_kline_info = Arc::new(kline_data.kline);
        
        // 测试 Strategy<Arc<T>> trait 实现 - 直接传递 Arc 值
        let signal = strategy.on_kline_update(arc_kline_info);
        println!("Generated signal from Arc<KlineInfo> (from WebSocket): {:?}", signal);
        
        // 验证信号属性
        assert_eq!(signal.symbol, "BTCUSDT");
        assert_eq!(signal.price, 46000.0);
        assert!(signal.timestamp > 0);
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use crate::dto::binance::websocket::KlineInfo;
    use std::time::Instant;

    // 创建测试用的 KlineInfo 数据
    fn create_test_kline_data() -> KlineInfo {
        KlineInfo {
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
        }
    }

    #[test]
    fn benchmark_sequential_vs_parallel() {
        let mut strategy = MacdStrategy::new(20).unwrap();
        let kline_data = create_test_kline_data();
        
        // 顺序计算
        let start = Instant::now();
        for _ in 0..1000 {
            let _hbfc_val = strategy.hbfc.next(&kline_data);
            let _ema_val = strategy.ema.next(&kline_data);
        }
        let sequential_time = start.elapsed();
        
        // 并行计算
        let start = Instant::now();
        for _ in 0..1000 {
            let _result = rayon::join(
                || strategy.hbfc.next(&kline_data),
                || strategy.ema.next(&kline_data),
            );
        }
        let parallel_time = start.elapsed();
        
        println!("Sequential: {:?}", sequential_time);
        println!("Parallel: {:?}", parallel_time);
        println!("Ratio: {:.2}x", parallel_time.as_nanos() as f64 / sequential_time.as_nanos() as f64);
    }
}

// 使用示例：展示如何在 WebSocket 消息处理中使用策略
pub async fn example_websocket_strategy_usage() -> Result<()> {
    let (ws_manager, mut message_rx) = create_websocket_manager().await?;
    
    // 创建策略实例
    let mut macd_strategy = MacdStrategy::new(20)?;
    
    // 启动 WebSocket 连接
    let kline_config = KlineConfig::new(
        "BTCUSDT", 
        "1m", 
        WebSocketBaseConfig {
            auto_reconnect: true,
            max_retries: 5,
            retry_delay_secs: 5,
            connection_timeout_secs: 10,
            message_timeout_secs: 30,
            enable_heartbeat: true,
            heartbeat_interval_secs: 30,
            tags: vec!["strategy".to_string()],
        }
    );
    
    ws_manager.start_kline(kline_config).await?;
    
    // 处理 WebSocket 消息
    while let Some(message) = message_rx.recv().await {
        match message {
            WebSocketMessage::Kline(kline_data) => {
                // 使用 kline 字段，它实现了所需的 traits
                let kline_info = Arc::new(kline_data.kline.clone());
                let signal = macd_strategy.on_kline_update(kline_info);
                
                // 修复信号类型匹配
                match signal.signal_type {
                    Some(SignalType::Buy) => {
                        println!("🟢 买入信号: {} @ {}", signal.symbol, signal.price);
                    }
                    Some(SignalType::Sell) => {
                        println!("🔴 卖出信号: {} @ {}", signal.symbol, signal.price);
                    }
                    None => {
                        println!("⏸️  持有信号: {}", signal.symbol);
                    }
                }
            }
            _ => {
                // 忽略其他类型的消息
            }
        }
    }
    
    Ok(())
}
