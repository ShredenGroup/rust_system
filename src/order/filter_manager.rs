use crate::common::enums::{Exchange, StrategyName};
use crate::exchange_api::binance::api::BinanceFuturesApi;
use crate::models::{Signal, TradingSignal, TradingSymbol};
use anyhow::Result;
use std::collections::HashMap;
use std::str::FromStr;
use tokio::sync::mpsc;

// 导入日志宏
use crate::{signal_log, order_log, error_log};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct PositionKey {
    exchange: Exchange,
    symbol: TradingSymbol,
    strategy: StrategyName,
}
impl PositionKey {
    pub fn new(exchange: Exchange, symbol: TradingSymbol, strategy: StrategyName) -> Self {
        PositionKey {
            exchange,
            symbol,
            strategy,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct Position {
    entry_price: f64,
    amount: f64,
    unrealized_pnl: f64,
    realized_pnl: Option<f64>,
    last_updated_time: u64,
    created_time: u64,
}
#[derive(Debug)]
pub struct PositionManager {
    positions: HashMap<PositionKey, Position>,
    all_pnl: f64,
    balance: f64,
}

impl PositionManager {
    pub fn new(balance: f64) -> Self {
        Self {
            positions: HashMap::new(),
            all_pnl: 0.0,
            balance,
        }
    }

    pub fn set_position(&mut self, k: PositionKey, v: Position) {
        self.positions.insert(k, v);
    }
    
    pub fn remove_position(&mut self, k: PositionKey) {
        self.positions.remove(&k);
    }
    
    pub fn update_position(&mut self, k: PositionKey, quantity: f64) {
        if let Some(position) = self.positions.get_mut(&k) {
            position.amount = quantity;
        }
    }
    
    pub fn get_position(&self, k: PositionKey) -> Option<Position> {
        self.positions.get(&k).cloned()
    }
    
    pub fn get_position_quantity(&self, k: PositionKey) -> f64 {
        self.positions
            .get(&k)
            .map(|position| position.amount)
            .unwrap_or(0.0)
    }
    
    pub fn get_position_by_key(&self, key: PositionKey) -> Option<Position> {
        self.positions.get(&key).cloned()
    }
    
    pub fn set_position_by_signal(&mut self, signal: &TradingSignal, quantity: f64) {
        // 从 TradingSignal 中提取信息创建 PositionKey
        let symbol = TradingSymbol::from_str(&signal.symbol).unwrap_or(TradingSymbol::BTCUSDT);
        let key = PositionKey::new(signal.exchange(), symbol, signal.strategy);
        
        let position = Position {
            entry_price: signal.latest_price,
            amount: quantity,
            unrealized_pnl: 0.0,
            realized_pnl: None,
            last_updated_time: signal.timestamp(),
            created_time: signal.timestamp(),
        };
        
        self.positions.insert(key, position);
    }
    
    pub fn remove_position_by_signal(&mut self, signal: &TradingSignal) {
        let symbol = TradingSymbol::from_str(&signal.symbol).unwrap_or(TradingSymbol::BTCUSDT);
        let key = PositionKey::new(signal.exchange(), symbol, signal.strategy);
        self.positions.remove(&key);
    }
    
    pub fn get_position_quantity_by_signal(&self, signal: &TradingSignal) -> f64 {
        let symbol = TradingSymbol::from_str(&signal.symbol).unwrap_or(TradingSymbol::BTCUSDT);
        let key = PositionKey::new(signal.exchange(), symbol, signal.strategy);
        self.get_position_quantity(key)
    }
}

pub struct SignalManager {
    pub position_manager: PositionManager,
    pub signal_receiver: mpsc::Receiver<TradingSignal>,
    binance_client: BinanceFuturesApi,
}

impl SignalManager {
    /// 创建新的 SignalManager，接受已创建的 BinanceFuturesApi 实例
    pub fn new_with_client(
        signal_receiver: mpsc::Receiver<TradingSignal>,
        position_manager: PositionManager,
        binance_client: BinanceFuturesApi,
    ) -> Self {
        Self {
            position_manager,
            signal_receiver,
            binance_client,
        }
    }

    /// 创建新的 SignalManager（保持向后兼容）
    pub fn new(
        signal_receiver: mpsc::Receiver<TradingSignal>,
        balance: f64,
        api_key: String,
        secret_key: String,
    ) -> Self {
        let binance_client = BinanceFuturesApi::new(api_key, secret_key);
        let position_manager = PositionManager::new(balance);
        Self {
            position_manager,
            signal_receiver,
            binance_client,
        }
    }

    pub async fn process_signals(&mut self) -> Result<()> {
        tracing::info!("🚀 SignalManager开始等待信号...");

        while let Some(signal) = self.signal_receiver.recv().await {
            signal_log!(info, "📥 接收到信号: 策略={:?}, 交易对={}, 方向={:?}",
                signal.strategy,
                signal.symbol,
                signal.side
            );

            // 直接处理信号，使用借用的 client
            let strategy = signal.strategy;
            let result = self.process_single_signal(signal).await;
            match &result {
                Ok(_) => signal_log!(info, "✅ 信号处理成功: 策略={:?}", strategy),
                Err(e) => error_log!(error, "❌ 信号处理失败: 策略={:?}, 错误: {}", strategy, e),
            }

            // 如果处理失败，可以选择是否继续处理下一个信号
            if result.is_err() {
                tracing::warn!("⚠️ 信号处理失败，继续处理下一个信号");
            }
        }

        tracing::info!("🎉 所有信号处理完成");
        Ok(())
    }

    async fn process_single_signal(&mut self, signal: TradingSignal) -> Result<()> {
        let strategy = signal.strategy;

        // 1. 检查信号类型并处理仓位
        let is_closing_signal = if let Signal::Market(market_signal) = &signal.signal {
            market_signal.is_closed
        } else {
            false
        };

        let original_position = if is_closing_signal {
            // 平仓信号：先保存原始仓位，然后清零
            let current_position = self.position_manager.get_position_quantity_by_signal(&signal);
            
            if current_position <= 0.0 {
                tracing::warn!(
                    "⚠️ 平仓信号但无仓位: 策略 {:?}, 交易对: {}, 当前仓位: {}",
                    strategy,
                    signal.symbol,
                    current_position
                );
                return Ok(()); // 没有仓位，直接返回
            }

            // 清零内存仓位
            self.position_manager.set_position_by_signal(&signal, 0.0);
            tracing::info!(
                "📤 处理平仓信号: 策略 {:?}, 交易对: {}, 原始仓位: {}, 清零仓位",
                strategy,
                signal.symbol,
                current_position
            );
            
            current_position // 保存原始仓位用于回滚
        } else {
            // 开仓信号：先检查是否已有仓位
            let current_position = self.position_manager.get_position_quantity_by_signal(&signal);
            
            if current_position > 0.0 {
                tracing::warn!(
                    "⚠️ 拒绝重复开仓: 策略 {:?}, 交易对: {}, 当前仓位: {}, 新信号数量: {}",
                    strategy,
                    signal.symbol,
                    current_position,
                    signal.quantity
                );
                return Ok(()); // 直接返回，不执行订单
            }

            // 没有仓位，可以开仓
            tracing::info!(
                "📤 处理开仓信号: 策略 {:?}, 交易对: {}, 设置仓位为 {}",
                strategy,
                signal.symbol,
                signal.quantity
            );
            
            // 先设置仓位
            self.position_manager.set_position_by_signal(&signal, signal.quantity);
            0.0 // 开仓信号没有原始仓位
        };

        // 2. 执行订单 - 使用借用的 client
        match self.binance_client.signal_to_order(&signal).await {
            Ok(order_ids) => {
                order_log!(info, "✅ 订单执行成功: 策略 {:?}, 交易对: {}, 方向: {:?}, 数量: {}, 订单ID: {:?}",
                    strategy,
                    signal.symbol,
                    signal.side,
                    signal.quantity,
                    order_ids
                );
                Ok(())
            }
            Err(e) => {
                // 订单执行失败，回滚仓位
                if is_closing_signal {
                    // 平仓失败：回滚到原始仓位
                    self.position_manager.set_position_by_signal(&signal, original_position);
                    tracing::error!(
                        "❌ 平仓订单执行失败，回滚仓位: 策略 {:?}, 交易对: {}, 回滚到: {}",
                        strategy,
                        signal.symbol,
                        original_position
                    );
                } else {
                    // 开仓失败：移除仓位
                    self.position_manager.remove_position_by_signal(&signal);
                    tracing::error!("❌ 开仓订单执行失败，移除仓位: 策略 {:?}, 交易对: {}", strategy, signal.symbol);
                }

                tracing::error!("❌ 订单执行失败: {}", e);
                Err(anyhow::anyhow!("Failed to place orders: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::config::user_config::load_binance_user_config;
    use crate::models::Side;

    #[tokio::test]
    async fn test_sequential_signal_processing() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let position_manager = PositionManager::new(10000.0); // 初始余额

        // 创建共享的API客户端
        let shared_api_client = BinanceFuturesApi::new(user_config.api_key, user_config.secret_key);

        let mut manager =
            SignalManager::new_with_client(signal_rx, position_manager, shared_api_client);

        // 创建多个测试信号
        let test_signals = vec![
            TradingSignal::new_market_signal(
                1,
                "BTCUSDT".to_string(),
                Side::Buy,
                StrategyName::MACD,
                0.001,
                Exchange::Binance,
                0,
                None,
                None,
                50000.0,
            ),
            TradingSignal::new_market_signal(
                2,
                "ETHUSDT".to_string(),
                Side::Buy,
                StrategyName::HBFC,
                0.01,
                Exchange::Binance,
                0,
                None,
                None,
                3000.0,
            ),
        ];

        // 顺序发送信号
        for signal in test_signals {
            signal_tx.send(signal).await.unwrap();
        }

        // 关闭发送端
        drop(signal_tx);

        // 运行信号处理
        manager.process_signals().await.unwrap();
    }

    #[tokio::test]
    async fn test_process_signals_market_only() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let position_manager = PositionManager::new(10000.0); // 初始余额

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, position_manager, shared_api_client);

        // 创建测试信号：只有市价单，无止损止盈
        let test_signal = TradingSignal::new_market_signal(
            1,                       // id
            "TURBOUSDT".to_string(), // symbol
            Side::Buy,               // side: 买入
            StrategyName::MACD,      // strategy
            10000.0,                 // quantity: 10000 (增加金额以避免API错误)
            Exchange::Binance,       // exchange
            0,                       // data_timestamp
            None,                    // profit_price: 无止盈
            None,                    // stop_price: 无止损
            0.5,                     // latest_price: 当前价格
        );

        println!("🧪 开始测试 process_signals 市价单功能...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", test_signal.symbol);
        println!("   方向: {:?}", test_signal.side);
        println!("   数量: {}", test_signal.quantity);
        println!("   策略: {:?}", test_signal.strategy);
        println!("   无止损止盈");

        // 发送信号
        signal_tx.send(test_signal).await.unwrap();

        // 关闭发送端，让接收端知道没有更多信号
        drop(signal_tx);

        // 运行信号处理
        let result = manager.process_signals().await;

        if result.is_ok() {
            println!("✅ process_signals 市价单测试成功！");

            // 等待一段时间让异步任务完成
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // 验证仓位是否被正确设置
            let test_signal = TradingSignal::new_market_signal(
                1,
                "TURBOUSDT".to_string(),
                Side::Buy,
                StrategyName::MACD,
                1000.0,
                Exchange::Binance,
                0,
                None,
                None,
                0.5,
            );
            let position = manager.position_manager.get_position_quantity_by_signal(&test_signal);
            println!(
                "📊 仓位设置成功: 策略 {:?}, 交易对: {}, 数量: {}",
                StrategyName::MACD,
                test_signal.symbol,
                position
            );
            assert_eq!(position, 10000.0, "仓位数量应该匹配信号数量");

            println!("🎉 测试通过！成功处理市价单信号并设置仓位");
        } else {
            let error = result.unwrap_err();
            println!("❌ process_signals 市价单测试失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_process_signals_with_stop_loss() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let position_manager = PositionManager::new(10000.0); // 初始余额

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, position_manager, shared_api_client);

        // 创建测试信号：市价单 + 止损单
        let test_signal = TradingSignal::new_market_signal(
            2,                       // id
            "TURBOUSDT".to_string(), // symbol
            Side::Buy,               // side: 买入
            StrategyName::HBFC,      // strategy (使用不同策略避免冲突)
            10000.0,                 // quantity: 10000
            Exchange::Binance,       // exchange
            0,                       // data_timestamp
            None,                    // profit_price: 无止盈
            Some(0.002),             // stop_price: 0.002美金止损
            0.5,                     // latest_price: 当前价格
        );

        println!("🧪 开始测试 process_signals 市价单+止损单功能...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", test_signal.symbol);
        println!("   方向: {:?}", test_signal.side);
        println!("   数量: {}", test_signal.quantity);
        println!("   策略: {:?}", test_signal.strategy);
        println!("   止损价: 0.002");
        println!("   无止盈");

        // 发送信号
        signal_tx.send(test_signal).await.unwrap();

        // 关闭发送端，让接收端知道没有更多信号
        drop(signal_tx);

        // 运行信号处理
        let result = manager.process_signals().await;

        if result.is_ok() {
            println!("✅ process_signals 市价单+止损单测试成功！");

            // 等待一段时间让异步任务完成
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // 验证仓位是否被正确设置
            let test_signal = TradingSignal::new_market_signal(
                2,
                "TURBOUSDT".to_string(),
                Side::Buy,
                StrategyName::HBFC,
                10000.0,
                Exchange::Binance,
                0,
                None,
                Some(0.002),
                0.5,
            );
            let position = manager.position_manager.get_position_quantity_by_signal(&test_signal);
            println!(
                "📊 仓位设置成功: 策略 {:?}, 交易对: {}, 数量: {}",
                StrategyName::HBFC,
                test_signal.symbol,
                position
            );
            assert_eq!(position, 10000.0, "仓位数量应该匹配信号数量");

            println!("🎉 测试通过！成功处理市价单+止损单信号并设置仓位");
        } else {
            let error = result.unwrap_err();
            println!("❌ process_signals 市价单+止损单测试失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_process_signals_reject_duplicate() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let position_manager = PositionManager::new(10000.0); // 初始余额

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, position_manager, shared_api_client);

        // 先设置一个仓位
        let initial_signal = TradingSignal::new_market_signal(
            3,
            "TURBOUSDT".to_string(),
            Side::Buy,
            StrategyName::MACD,
            5000.0,
            Exchange::Binance,
            0,
            None,
            None,
            0.5,
        );
        manager.position_manager.set_position_by_signal(&initial_signal, 5000.0);

        // 创建测试信号：尝试重复开仓
        let duplicate_signal = TradingSignal::new_market_signal(
            4,                       // id
            "TURBOUSDT".to_string(), // symbol
            Side::Buy,               // side: 买入
            StrategyName::MACD,      // strategy (已有仓位的策略)
            10000.0,                 // quantity: 10000
            Exchange::Binance,       // exchange
            0,                       // data_timestamp
            None,                    // profit_price: 无止盈
            None,                    // stop_price: 无止损
            0.5,                     // latest_price: 当前价格
        );

        println!("🧪 开始测试 process_signals 重复开仓拒绝功能...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", duplicate_signal.symbol);
        println!("   方向: {:?}", duplicate_signal.side);
        println!("   数量: {}", duplicate_signal.quantity);
        println!("   策略: {:?} (已有仓位)", duplicate_signal.strategy);
        println!("   当前仓位: 5000.0");

        // 发送信号
        signal_tx.send(duplicate_signal).await.unwrap();

        // 关闭发送端，让接收端知道没有更多信号
        drop(signal_tx);

        // 运行信号处理
        let result = manager.process_signals().await;

        if result.is_ok() {
            println!("✅ process_signals 重复开仓拒绝测试成功！");

            // 验证仓位没有被修改
            let check_signal = TradingSignal::new_market_signal(
                3,
                "TURBOUSDT".to_string(),
                Side::Buy,
                StrategyName::MACD,
                5000.0,
                Exchange::Binance,
                0,
                None,
                None,
                0.5,
            );
            let position = manager.position_manager.get_position_quantity_by_signal(&check_signal);
            println!(
                "📊 仓位保持不变: 策略 {:?}, 交易对: {}, 数量: {}",
                StrategyName::MACD,
                check_signal.symbol,
                position
            );
            assert_eq!(position, 5000.0, "仓位数量应该保持不变");

            println!("🎉 测试通过！成功拒绝重复开仓，仓位保持不变");
        } else {
            let error = result.unwrap_err();
            println!("❌ process_signals 重复开仓拒绝测试失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_process_signals_close_position() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let position_manager = PositionManager::new(10000.0); // 初始余额

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, position_manager, shared_api_client);

        // 先设置一个仓位（模拟已有持仓）
        let initial_signal = TradingSignal::new_market_signal(
            1,
            "TURBOUSDT".to_string(),
            Side::Buy,
            StrategyName::BOLLINGER,
            10000.0,
            Exchange::Binance,
            0,
            None,
            None,
            0.5,
        );
        manager.position_manager.set_position_by_signal(&initial_signal, 10000.0);
        println!(
            "📊 初始仓位设置: 策略 {:?}, 交易对: {}, 数量: 10000.0",
            StrategyName::BOLLINGER,
            initial_signal.symbol
        );

        // 创建平仓信号：卖出平多（使用现有的构造方法）
        let close_signal = TradingSignal::new_close_signal(
            1,                       // id
            "TURBOUSDT".to_string(), // symbol
            1,                       // current_position: 1 表示多头
            StrategyName::BOLLINGER, // strategy
            10000.0,                 // quantity
            Exchange::Binance,       // exchange
            0.5,                     // latest_price
        );

        println!("🧪 开始测试 process_signals 平仓信号功能...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", close_signal.symbol);
        println!("   方向: {:?}", close_signal.side);
        println!("   数量: {}", close_signal.quantity);
        println!("   策略: {:?}", close_signal.strategy);
        println!("   信号类型: 平仓信号 (is_closed = true)");
        println!("   当前仓位: 10000.0");

        // 发送平仓信号
        signal_tx.send(close_signal).await.unwrap();

        // 关闭发送端，让接收端知道没有更多信号
        drop(signal_tx);

        // 运行信号处理
        let result = manager.process_signals().await;

        // 等待一段时间让异步任务完成
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // 验证仓位状态
        let check_signal = TradingSignal::new_close_signal(
            1,
            "TURBOUSDT".to_string(),
            1,
            StrategyName::BOLLINGER,
            10000.0,
            Exchange::Binance,
            0.5,
        );
        let position = manager.position_manager.get_position_quantity_by_signal(&check_signal);
        
        if result.is_ok() {
            // 平仓成功：仓位应该为 0
            println!("✅ process_signals 平仓信号测试成功！");
            println!(
                "📊 平仓成功: 策略 {:?}, 交易对: {}, 数量: {}",
                StrategyName::BOLLINGER,
                check_signal.symbol,
                position
            );
            assert_eq!(position, 0.0, "平仓成功后仓位应该为 0");
            println!("🎉 测试通过！成功处理平仓信号并将仓位设置为 0");
        } else {
            // 平仓失败：仓位应该回滚到原始值
            let error = result.unwrap_err();
            println!("✅ process_signals 平仓失败回滚测试成功！");
            println!(
                "📊 平仓失败回滚: 策略 {:?}, 交易对: {}, 数量: {}",
                StrategyName::BOLLINGER,
                check_signal.symbol,
                position
            );
            assert_eq!(position, 10000.0, "平仓失败后仓位应该回滚到原始值");
            println!("🎉 测试通过！平仓失败后成功回滚仓位到原始值");
        }
    }

    #[tokio::test]
    async fn test_process_signals_close_position_without_position() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let position_manager = PositionManager::new(10000.0); // 初始余额

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, position_manager, shared_api_client);

        // 不设置初始仓位（模拟没有持仓的情况）

        // 创建平仓信号：尝试平仓但没有持仓（使用现有的构造方法）
        let close_signal = TradingSignal::new_close_signal(
            2,                       // id
            "TURBOUSDT".to_string(), // symbol
            1,                       // current_position: 1 表示多头
            StrategyName::BOLLINGER, // strategy
            10000.0,                 // quantity
            Exchange::Binance,       // exchange
            0.5,                     // latest_price
        );

        println!("🧪 开始测试 process_signals 无持仓平仓信号功能...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", close_signal.symbol);
        println!("   方向: {:?}", close_signal.side);
        println!("   数量: {}", close_signal.quantity);
        println!("   策略: {:?}", close_signal.strategy);
        println!("   信号类型: 平仓信号 (is_closed = true)");
        println!("   当前仓位: 无持仓");

        // 发送平仓信号
        signal_tx.send(close_signal).await.unwrap();

        // 关闭发送端，让接收端知道没有更多信号
        drop(signal_tx);

        // 运行信号处理
        let result = manager.process_signals().await;

        if result.is_ok() {
            println!("✅ process_signals 无持仓平仓信号测试成功！");

            // 等待一段时间让异步任务完成
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // 验证仓位是否被正确设置为 0
            let close_signal = TradingSignal::new_close_signal(
                2,
                "TURBOUSDT".to_string(),
                1,
                StrategyName::BOLLINGER,
                10000.0,
                Exchange::Binance,
                0.5,
            );
            let position = manager.position_manager.get_position_quantity_by_signal(&close_signal);
            println!(
                "📊 仓位设置成功: 策略 {:?}, 交易对: {}, 数量: {}",
                StrategyName::BOLLINGER,
                close_signal.symbol,
                position
            );
            assert_eq!(position, 0.0, "平仓信号应该将仓位设置为 0");

            println!("🎉 测试通过！成功处理无持仓的平仓信号");
        } else {
            let error = result.unwrap_err();
            println!("❌ process_signals 无持仓平仓信号测试失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_process_signals_close_position_failure_rollback() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let position_manager = PositionManager::new(10000.0); // 初始余额

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, position_manager, shared_api_client);

        // 先设置一个仓位（模拟已有持仓）
        let initial_signal = TradingSignal::new_market_signal(
            1,
            "TURBOUSDT".to_string(),
            Side::Buy,
            StrategyName::BOLLINGER,
            10000.0,
            Exchange::Binance,
            0,
            None,
            None,
            0.5,
        );
        manager.position_manager.set_position_by_signal(&initial_signal, 10000.0);
        println!(
            "📊 初始仓位设置: 策略 {:?}, 交易对: {}, 数量: 10000.0",
            StrategyName::BOLLINGER,
            initial_signal.symbol
        );

        // 创建平仓信号：使用一个会导致API错误的数量（比如0.001，太小）
        let close_signal = TradingSignal::new_close_signal(
            1,                       // id
            "TURBOUSDT".to_string(), // symbol
            1,                       // current_position: 1 表示多头
            StrategyName::BOLLINGER, // strategy
            0.001,                   // quantity: 使用很小的数量来触发API错误
            Exchange::Binance,       // exchange
            0.5,                     // latest_price
        );

        println!("🧪 开始测试 process_signals 平仓失败回滚功能...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", close_signal.symbol);
        println!("   方向: {:?}", close_signal.side);
        println!("   数量: {}", close_signal.quantity);
        println!("   策略: {:?}", close_signal.strategy);
        println!("   信号类型: 平仓信号 (is_closed = true)");
        println!("   当前仓位: 10000.0");
        println!("   预期: 平仓失败，仓位回滚到 10000.0");

        // 发送平仓信号
        signal_tx.send(close_signal).await.unwrap();

        // 关闭发送端，让接收端知道没有更多信号
        drop(signal_tx);

        // 运行信号处理
        let result = manager.process_signals().await;

        // 注意：这里我们期望结果是错误，因为订单会失败
        if result.is_err() {
            println!("✅ process_signals 平仓失败回滚测试成功！");

            // 等待一段时间让异步任务完成
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // 验证仓位是否被正确回滚到原始值
            let check_signal = TradingSignal::new_close_signal(
                1,
                "TURBOUSDT".to_string(),
                1,
                StrategyName::BOLLINGER,
                10000.0,
                Exchange::Binance,
                0.5,
            );
            let position = manager.position_manager.get_position_quantity_by_signal(&check_signal);
            println!(
                "📊 仓位回滚成功: 策略 {:?}, 交易对: {}, 数量: {}",
                StrategyName::BOLLINGER,
                check_signal.symbol,
                position
            );
            assert_eq!(position, 10000.0, "平仓失败后仓位应该回滚到原始值");

            println!("🎉 测试通过！成功处理平仓失败并回滚仓位");
        } else {
            println!("❌ 预期平仓应该失败，但实际成功了");
            panic!("测试逻辑错误：平仓应该失败但没有失败");
        }
    }
}
