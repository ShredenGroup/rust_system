use crate::common::enums::{Exchange, StrategyName};
use crate::exchange_api::binance::api::BinanceFuturesApi;
use crate::models::{Signal, TradingSignal, TradingSymbol};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone)]
pub struct Position {
    entry_price: f64,
    amount: f64,
    unrealized_pnl: f64,
    realized_pnl: Option<f64>,
    last_updated_time: u64,
    created_time: u64,
}

#[derive(Debug, Clone)]
pub struct PositionManager {
    positions: Arc<RwLock<HashMap<PositionKey, Position>>>,
}

impl PositionManager {
    pub fn new() -> Self {
        Self {
            positions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn set_position(&self, k: PositionKey, v: Position) {
        let mut positions = self.positions.write().await;
        positions.insert(k, v);
    }
    pub async fn remove_position(&self,k:PositionKey){
        let mut positions = self.positions.write().await;
        positions.remove(&k);
    }
    pub async fn update_position(&self,k:PositionKey,quantity:f64){
        let mut positions = self.positions.write().await;
        if let Some(position) = positions.get_mut(&k) {
            position.amount = quantity;
        }
    }
    pub async fn get_position(&self,k:PositionKey) -> Option<Position>{
        let positions = self.positions.read().await;
        positions.get(&k).cloned()
    }
    pub async fn get_position_quantity(&self,k:PositionKey) -> f64{
        let positions=self.positions.read().await;
        positions.get(&k).map(|position| position.amount).unwrap_or(0.0)
    }
}

pub struct SignalManager {
    pub open_position: Arc<RwLock<HashMap<StrategyName, f64>>>,
    pub signal_receiver: mpsc::Receiver<TradingSignal>,
    binance_client: BinanceFuturesApi,
}

impl SignalManager {
    /// 创建新的 SignalManager，接受已创建的 BinanceFuturesApi 实例
    pub fn new_with_client(
        signal_receiver: mpsc::Receiver<TradingSignal>,
        open_position: Arc<RwLock<HashMap<StrategyName, f64>>>,
        binance_client: BinanceFuturesApi,
    ) -> Self {
        Self {
            open_position,
            signal_receiver,
            binance_client,
        }
    }

    /// 创建新的 SignalManager（保持向后兼容）
    pub fn new(
        signal_receiver: mpsc::Receiver<TradingSignal>,
        open_position: Arc<RwLock<HashMap<StrategyName, f64>>>,
        api_key: String,
        secret_key: String,
    ) -> Self {
        let binance_client = BinanceFuturesApi::new(api_key, secret_key);
        Self {
            open_position,
            signal_receiver,
            binance_client,
        }
    }

    pub async fn process_signals(&mut self) -> Result<()> {
        tracing::info!("🚀 SignalManager开始等待信号...");

        while let Some(signal) = self.signal_receiver.recv().await {
            tracing::info!(
                "📥 接收到信号: 策略={:?}, 交易对={}, 方向={:?}",
                signal.strategy,
                signal.symbol,
                signal.side
            );

            // 直接处理信号，使用借用的 client
            let strategy = signal.strategy;
            let result = self.process_single_signal(signal).await;
            match &result {
                Ok(_) => tracing::info!("✅ 信号处理成功: 策略={:?}", strategy),
                Err(e) => tracing::error!("❌ 信号处理失败: 策略={:?}, 错误: {}", strategy, e),
            }

            // 如果处理失败，可以选择是否继续处理下一个信号
            if result.is_err() {
                tracing::warn!("⚠️ 信号处理失败，继续处理下一个信号");
            }
        }

        tracing::info!("🎉 所有信号处理完成");
        Ok(())
    }

    async fn process_single_signal(&self, signal: TradingSignal) -> Result<()> {
        let strategy = signal.strategy;

        // 1. 直接更新仓位（不再检查是否已有仓位）
        {
            let mut positions = self.open_position.write().await;

            // 检查信号类型
            if let Signal::Market(market_signal) = &signal.signal {
                if market_signal.is_closed {
                    // 平仓信号：设置仓位为 0
                    positions.insert(strategy, 0.0);
                    tracing::info!("📤 处理平仓信号: 策略 {:?}, 设置仓位为 0", strategy);
                } else {
                    // 开仓信号：设置仓位
                    positions.insert(strategy, signal.quantity);
                    tracing::info!(
                        "📤 处理开仓信号: 策略 {:?}, 设置仓位为 {}",
                        strategy,
                        signal.quantity
                    );
                }
            } else {
                // 其他类型信号：设置仓位
                positions.insert(strategy, signal.quantity);
                tracing::info!(
                    "📤 处理其他信号: 策略 {:?}, 设置仓位为 {}",
                    strategy,
                    signal.quantity
                );
            }
        }

        // 2. 执行订单 - 使用借用的 client
        match self.binance_client.signal_to_order(&signal).await {
            Ok(order_ids) => {
                tracing::info!(
                    "✅ 订单执行成功: 策略 {:?}, 交易对: {}, 方向: {:?}, 数量: {}, 订单ID: {:?}",
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
                let mut positions = self.open_position.write().await;
                positions.remove(&strategy);
                tracing::error!("❌ 订单执行失败，移除仓位: 策略 {:?}", strategy);

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
        let positions = Arc::new(RwLock::new(HashMap::new()));

        // 创建共享的API客户端
        let shared_api_client = BinanceFuturesApi::new(user_config.api_key, user_config.secret_key);

        let mut manager =
            SignalManager::new_with_client(signal_rx, positions.clone(), shared_api_client);

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
        let positions = Arc::new(RwLock::new(HashMap::new()));

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, positions.clone(), shared_api_client);

        // 创建测试信号：只有市价单，无止损止盈
        let test_signal = TradingSignal::new_market_signal(
            1,                       // id
            "TURBOUSDT".to_string(), // symbol
            Side::Buy,               // side: 买入
            StrategyName::MACD,      // strategy
            1000.0,                  // quantity: 10000
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
            let positions_guard = positions.read().await;
            if let Some(position) = positions_guard.get(&StrategyName::MACD) {
                println!(
                    "📊 仓位设置成功: 策略 {:?}, 数量: {}",
                    StrategyName::MACD,
                    position
                );
                assert_eq!(*position, 10000.0, "仓位数量应该匹配信号数量");
            } else {
                println!("❌ 仓位未找到，当前所有仓位: {:?}", *positions_guard);
                panic!("仓位应该被设置");
            }

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
        let positions = Arc::new(RwLock::new(HashMap::new()));

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, positions.clone(), shared_api_client);

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
            let positions_guard = positions.read().await;
            if let Some(position) = positions_guard.get(&StrategyName::HBFC) {
                println!(
                    "📊 仓位设置成功: 策略 {:?}, 数量: {}",
                    StrategyName::HBFC,
                    position
                );
                assert_eq!(*position, 10000.0, "仓位数量应该匹配信号数量");
            } else {
                println!("❌ 仓位未找到，当前所有仓位: {:?}", *positions_guard);
                panic!("仓位应该被设置");
            }

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
        let positions = Arc::new(RwLock::new(HashMap::new()));

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, positions.clone(), shared_api_client);

        // 先设置一个仓位
        {
            let mut positions_guard = positions.write().await;
            positions_guard.insert(StrategyName::MACD, 5000.0);
        }

        // 创建测试信号：尝试在已有仓位的情况下再次下单
        let test_signal = TradingSignal::new_market_signal(
            3,                       // id
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

        println!("🧪 开始测试 process_signals 重复信号拒绝功能...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", test_signal.symbol);
        println!("   方向: {:?}", test_signal.side);
        println!("   数量: {}", test_signal.quantity);
        println!("   策略: {:?} (已有仓位)", test_signal.strategy);
        println!("   当前仓位: 5000.0");

        // 发送信号
        signal_tx.send(test_signal).await.unwrap();

        // 关闭发送端，让接收端知道没有更多信号
        drop(signal_tx);

        // 运行信号处理
        let result = manager.process_signals().await;

        if result.is_ok() {
            println!("✅ process_signals 重复信号拒绝测试成功！");

            // 验证仓位没有被修改
            let positions_guard = positions.read().await;
            if let Some(position) = positions_guard.get(&StrategyName::MACD) {
                println!(
                    "📊 仓位保持不变: 策略 {:?}, 数量: {}",
                    StrategyName::MACD,
                    position
                );
                assert_eq!(*position, 5000.0, "仓位数量应该保持不变");
            } else {
                panic!("仓位应该存在且保持不变");
            }

            println!("🎉 测试通过！成功拒绝重复信号，仓位保持不变");
        } else {
            let error = result.unwrap_err();
            println!("❌ process_signals 重复信号拒绝测试失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_process_signals_close_position() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let positions = Arc::new(RwLock::new(HashMap::new()));

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, positions.clone(), shared_api_client);

        // 先设置一个仓位（模拟已有持仓）
        {
            let mut positions_guard = positions.write().await;
            positions_guard.insert(StrategyName::BOLLINGER, 10000.0);
            println!(
                "📊 初始仓位设置: 策略 {:?}, 数量: 10000.0",
                StrategyName::BOLLINGER
            );
        }

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

        if result.is_ok() {
            println!("✅ process_signals 平仓信号测试成功！");

            // 等待一段时间让异步任务完成
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

            // 验证仓位是否被正确设置为 0（平仓后）
            let positions_guard = positions.read().await;
            if let Some(position) = positions_guard.get(&StrategyName::BOLLINGER) {
                println!(
                    "📊 仓位更新成功: 策略 {:?}, 数量: {}",
                    StrategyName::BOLLINGER,
                    position
                );
                assert_eq!(*position, 0.0, "平仓后仓位应该为 0");
            } else {
                println!("❌ 仓位未找到，当前所有仓位: {:?}", *positions_guard);
                panic!("仓位应该存在且被设置为 0");
            }

            println!("🎉 测试通过！成功处理平仓信号并将仓位设置为 0");
        } else {
            let error = result.unwrap_err();
            println!("❌ process_signals 平仓信号测试失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_process_signals_close_position_without_position() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");

        let (signal_tx, signal_rx) = mpsc::channel(100);
        let positions = Arc::new(RwLock::new(HashMap::new()));

        // 创建共享的API客户端
        let shared_api_client =
            BinanceFuturesApi::new(user_config.api_key.clone(), user_config.secret_key.clone());

        let mut manager =
            SignalManager::new_with_client(signal_rx, positions.clone(), shared_api_client);

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
            let positions_guard = positions.read().await;
            if let Some(position) = positions_guard.get(&StrategyName::BOLLINGER) {
                println!(
                    "📊 仓位设置成功: 策略 {:?}, 数量: {}",
                    StrategyName::BOLLINGER,
                    position
                );
                assert_eq!(*position, 0.0, "平仓信号应该将仓位设置为 0");
            } else {
                println!("❌ 仓位未找到，当前所有仓位: {:?}", *positions_guard);
                panic!("平仓信号应该创建仓位记录并设置为 0");
            }

            println!("🎉 测试通过！成功处理无持仓的平仓信号");
        } else {
            let error = result.unwrap_err();
            println!("❌ process_signals 无持仓平仓信号测试失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }
}
