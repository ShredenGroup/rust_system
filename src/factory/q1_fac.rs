use crate::{
    common::{
        config::ws_config::{KlineConfig, WebSocketBaseConfig},
        config::user_config::load_binance_user_config,
        TradingSymbol,
    },
    exchange_api::binance::{
        ws_manager::{create_websocket_manager, WebSocketMessage},
        api_manager::{create_api_manager, ApiMessage},
    },
    strategy::{
        q1::Q1Strategy,
        strategy_manager::{StrategyManager, StrategyEnum, IdGenerator},
    },
    order::filter_manager::SignalManager,
};

use tokio;
use anyhow::Result;

use std::time::Instant;
use tracing::{info, debug, error};
use tracing_subscriber::EnvFilter;
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use crate::dto::unified::UnifiedKlineData;

/// Q1策略工厂
pub struct Q1Factory;

impl Q1Factory {
    /// 设置日志系统
    pub fn setup_logging() -> Result<()> {
        // 创建logs目录
        let log_dir = "logs";
        if !Path::new(log_dir).exists() {
            fs::create_dir(log_dir)?;
        }

        // 配置日志过滤器
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,q1_factory=debug"));

        // 配置文件输出
        let file_appender = tracing_appender::rolling::daily(log_dir, "q1_factory.log");

        // 初始化日志系统
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(file_appender)
            .init();

        info!("🚀 Q1策略工厂启动");
        info!("📁 日志文件保存在: {}", log_dir);
        
        Ok(())
    }

    /// 运行Q1策略
    pub async fn run_q1_strategy() -> Result<()> {
        info!("🚀 启动Q1策略");
        info!("{}", "=".repeat(80));

        // 定义要交易的币种
        let trading_symbols = vec![
            TradingSymbol::BTCUSDT,   // 比特币
            TradingSymbol::ETHUSDT,   // 以太坊
            TradingSymbol::PEPEUSDT,  // 映射到 "1000PEPEUSDT"
            TradingSymbol::NEIROUSDT,
            TradingSymbol::ONDOUSDT,  // ONDO
            TradingSymbol::AAVEUSDT,  // AAVE
        ];
        
        info!("📊 交易币种列表:");
        for symbol in &trading_symbols {
            info!("   • {}", symbol.as_str());
        }

        // 加载API配置
        let user_config = load_binance_user_config()?;
        info!("✅ 已加载用户配置");

        // 创建信号处理通道
        let (signal_tx, signal_rx) = mpsc::channel(1000);
        let positions = Arc::new(RwLock::new(HashMap::new()));
        
        // 创建API管理器
        let (api_manager, mut api_rx) = create_api_manager(
            user_config.api_key.clone(),
            user_config.secret_key.clone(),
        ).await?;
        info!("✅ API管理器创建成功");

        // 从API管理器获取共享的BinanceFuturesApi实例
        let shared_api_client = api_manager.get_api_client();
        
        // 创建SignalManager，使用共享的API实例
        let mut signal_manager = SignalManager::new_with_client(
            signal_rx,
            positions.clone(),
            shared_api_client,
        );
        info!("✅ 信号管理器创建成功（使用共享API实例）");

        // 启动信号处理任务
        let signal_manager_handle = tokio::spawn(async move {
            info!("🚀 启动信号处理任务");
            if let Err(e) = signal_manager.process_signals().await {
                eprintln!("❌ 信号处理任务失败: {}", e);
            }
        });

        // 创建WebSocket管理器
        let (ws_manager, mut ws_rx) = create_websocket_manager().await?;
        info!("✅ WebSocket管理器创建成功");

        // 创建策略管理器相关的通道
        let (strategy_data_tx, strategy_data_rx) = mpsc::channel::<Arc<UnifiedKlineData>>(1000);
        
        let signal_tx_clone = signal_tx.clone();
        
        // 创建ID生成器
        let id_generator = Arc::new(IdGenerator::new((1, 1000000))?);
        
        // 创建策略管理器
        let mut strategy_manager = StrategyManager::new(
            strategy_data_rx,
            signal_tx_clone,
            id_generator.clone(),
        );
        
        // 为每个币种添加Q1策略
        info!("🎯 为各币种配置Q1策略:");
        for symbol in &trading_symbols {
            // 根据币种设置不同的参数
            let (break_period, ema_period, profit_period, atr_period, atr_multiplier) = match symbol {
                TradingSymbol::BTCUSDT => (
                    50,     // 更长的突破周期，减少假突破
                    240,    // EMA周期保持不变
                    15,     // 更长的止盈周期，让利润奔跑
                    20,     // ATR周期保持不变
                    2.5,    // 较小的ATR倍数，因为BTC波动较小
                ),
                TradingSymbol::ETHUSDT => (
                    45,     // 较长的突破周期
                    240,    // EMA周期保持不变
                    12,     // 中等止盈周期
                    20,     // ATR周期保持不变
                    2.8,    // 中等ATR倍数
                ),
                TradingSymbol::ONDOUSDT => (
                    30,     // 较短的突破周期，因为波动较大
                    240,    // EMA周期保持不变
                    8,      // 较短的止盈周期，快速获利
                    20,     // ATR周期保持不变
                    3.5,    // 较大的ATR倍数，因为波动较大
                ),
                TradingSymbol::AAVEUSDT => (
                    32,     // 中短突破周期
                    240,    // EMA周期保持不变
                    9,      // 中短止盈周期
                    20,     // ATR周期保持不变
                    3.2,    // 较大的ATR倍数
                ),
                _ => (
                    35,     // 默认突破周期
                    240,    // 默认EMA周期
                    10,     // 默认止盈周期
                    20,     // 默认ATR周期
                    3.0,    // 默认ATR倍数
                ),
            };

            // 创建Q1策略实例
            let q1_strategy = Q1Strategy::new(
                break_period,
                ema_period,
                profit_period,
                atr_period,
                atr_multiplier,
                None,  // symbol: 由策略管理器设置
            )?;
            let strategy_enum = StrategyEnum::Q1(q1_strategy);
            
            // 添加策略到管理器（会自动设置symbol）
            strategy_manager.add_strategy(symbol.clone(), strategy_enum).await?;
            info!("   ✅ {} - Q1策略配置完成", symbol.as_str());
            info!("      • 突破周期: {}", break_period);
            info!("      • EMA周期: {}", ema_period);
            info!("      • 止盈周期: {}", profit_period);
            info!("      • ATR周期: {}", atr_period);
            info!("      • ATR倍数: {:.1}", atr_multiplier);
        }

        // 启动策略管理器任务
        let strategy_manager_handle = tokio::spawn(async move {
            info!("🚀 启动策略管理器任务");
            if let Err(e) = strategy_manager.run().await {
                error!("❌ 策略管理器运行失败: {}", e);
            }
        });

        // 获取所有币种的历史K线数据进行初始化
        info!("📊 获取历史K线数据进行初始化...");
        for symbol in &trading_symbols {
            info!("   📈 获取 {} 历史数据", symbol.as_str());
            api_manager.get_history_klines(
                symbol.as_str().to_string(),
                "1h".to_string(),
                None,
                None,
                Some("241".to_string()),  // 获取足够的K线用于初始化（240 EMA需要）
            ).await?;

            // 等待并处理历史数据
            if let Some(message) = api_rx.recv().await {
                match message {
                    ApiMessage::Kline(kline_data_list) => {
                        info!("   📈 收到 {} 历史K线数据: {} 根", symbol.as_str(), kline_data_list.len());
                        
                        for kline in kline_data_list.iter() {
                            // 设置symbol字段（API数据需要手动设置）
                            let mut api_kline = kline.clone();
                            api_kline.symbol = symbol.clone();
                            
                            // 包装为统一数据类型
                            let unified_data = UnifiedKlineData::Api(api_kline);
                            
                            // 发送数据到策略管理器
                            if let Err(e) = strategy_data_tx.send(Arc::new(unified_data)).await {
                                error!("❌ 发送历史数据到策略管理器失败: {}", e);
                            } else {
                                debug!("📤 历史数据已发送到策略管理器: {} - 价格={:.6}", symbol.as_str(), kline.close);
                            }
                        }
                        info!("   ✅ {} 历史数据初始化完成", symbol.as_str());
                    }
                }
            }
        }

        info!("{}", "=".repeat(80));
        info!("🔄 开始实时数据处理");

        // 配置WebSocket连接 - 为所有币种
        let mut ws_configs = Vec::new();
        for symbol in &trading_symbols {
            let symbol_str = symbol.as_str().to_lowercase();
            let interval = "1h";
            
            let kline_config = KlineConfig::new(
                &symbol_str,
                interval,
                WebSocketBaseConfig {
                    auto_reconnect: true,
                    max_retries: 5,
                    retry_delay_secs: 5,
                    connection_timeout_secs: 10,
                    message_timeout_secs: 30,
                    enable_heartbeat: true,
                    heartbeat_interval_secs: 30,
                    tags: vec![format!("q1_multi_{}", symbol_str)],
                },
            );
            
            ws_configs.push((symbol.clone(), kline_config));
        }

        // 启动所有WebSocket连接
        for (symbol, config) in &ws_configs {
            info!("🔌 尝试建立WebSocket连接: {}/1m", symbol.as_str());
            match ws_manager.start_kline(config.clone()).await {
                Ok(_) => {
                    info!("✅ {} WebSocket连接已建立", symbol.as_str());
                }
                Err(e) => {
                    error!("❌ {} WebSocket连接失败: {}", symbol.as_str(), e);
                    return Err(anyhow::anyhow!("{} WebSocket连接失败: {}", symbol.as_str(), e));
                }
            }
        }

        // 统计变量
        let mut message_count = 0;
        let mut kline_count_by_symbol: HashMap<String, usize> = HashMap::new();
        let signal_count = 0;
        let mut total_latency = 0.0f64;

        // 处理实时数据
        let start_time = Instant::now();
        info!("🎯 开始接收实时K线数据...");
        
        while let Some(message) = ws_rx.recv().await {
            let ws_received_time = Instant::now();
            message_count += 1;

            // 每100条消息打印一次统计
            if message_count % 100 == 0 {
                info!("📊 统计信息: 总消息数={}, 信号数量={}", message_count, signal_count);
                for (symbol, count) in &kline_count_by_symbol {
                    info!("   {} K线数量: {}", symbol, count);
                }
            }

            match message {
                WebSocketMessage::Kline(kline_data) => {
                    let symbol_str = kline_data.symbol.as_str();
                    *kline_count_by_symbol.entry(symbol_str.to_string()).or_insert(0) += 1;
                    
                    let kline_info = &kline_data.kline;
                    debug!("📈 收到K线数据: {}, 价格={:.6}, 完成={}", 
                        symbol_str, kline_info.close_price, kline_info.is_closed);

                    // 发送数据到策略管理器
                    let strategy_start_time = Instant::now();
                    let ws_kline_data = (*kline_data).clone();
                    let unified_data = UnifiedKlineData::WebSocket(ws_kline_data);
                    if let Err(e) = strategy_data_tx.send(Arc::new(unified_data)).await {
                        error!("❌ 发送数据到策略管理器失败: {}", e);
                    } else {
                        let strategy_latency = strategy_start_time.elapsed().as_secs_f64() * 1000.0;
                        total_latency += strategy_latency;
                        
                        debug!("📤 数据已发送到策略管理器, 延迟: {:.3}ms", strategy_latency);
                    }

                    let total_processing_time = ws_received_time.elapsed().as_secs_f64() * 1000.0;
                    debug!("   总处理延迟: {:.3} ms", total_processing_time);
                }
                _ => {}
            }

            // 每1000条消息打印性能统计
            if message_count % 1000 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let avg_latency = if message_count > 0 { total_latency / message_count as f64 } else { 0.0 };
                
                info!("📊 性能统计:");
                info!("   处理消息数: {}, 信号生成数: {}", message_count, signal_count);
                info!("   运行时间: {:.2}秒", elapsed);
                info!("   消息处理率: {:.2}条/秒", message_count as f64 / elapsed);
                info!("   平均延迟: {:.3}ms", avg_latency);
                
                for (symbol, count) in &kline_count_by_symbol {
                    info!("   {} 处理速率: {:.2}条/秒", symbol, *count as f64 / elapsed);
                }
            }
        }

        info!("🏁 多币种Q1策略结束");
        info!("最终统计: 总消息数={}, 信号数量={}", message_count, signal_count);
        for (symbol, count) in &kline_count_by_symbol {
            info!("   {} 最终K线数量: {}", symbol, count);
        }

        // 等待所有任务完成
        info!("⏳ 等待所有任务完成...");
        
        if let Err(e) = signal_manager_handle.await {
            eprintln!("❌ 信号处理任务异常: {}", e);
        }

        if let Err(e) = strategy_manager_handle.await {
            eprintln!("❌ 策略管理器任务异常: {:?}", e);
        }

        info!("✅ 所有任务已完成");
        Ok(())
    }
}

