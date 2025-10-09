use crate::{
    common::{
        config::ws_config::{KlineConfig, WebSocketBaseConfig},
        config::user_config::load_binance_user_config,
        simple_logging::{SimpleLoggingManager, SimpleLoggingConfig},
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
    order::filter_manager::{SignalManager, PositionManager},
};

use tokio;
use anyhow::Result;

use tracing::{info, error};
use std::sync::Arc;
use tokio::sync::mpsc;
use crate::dto::unified::UnifiedKlineData;


/// Q1策略工厂
pub struct Q1Factory;

impl Q1Factory {
    /// 设置日志系统
    pub fn setup_logging() -> Result<()> {
        let config = SimpleLoggingConfig {
            log_dir: "logs".to_string(),
            enable_console: true,
        };
        
        let logging_manager = SimpleLoggingManager::new(config);
        logging_manager.init()?;
        
        info!("🚀 Q1策略工厂启动");
        
        Ok(())
    }

    /// 运行Q1策略
    pub async fn run_q1_strategy() -> Result<()> {
        info!("🚀 启动Q1策略");

        // 定义要交易的币种
        let trading_symbols = vec![
            TradingSymbol::BTCUSDT,   // 比特币
            TradingSymbol::ETHUSDT,   // 以太坊
            TradingSymbol::PEPEUSDT,  // 映射到 "1000PEPEUSDT"
            TradingSymbol::NEIROUSDT,
            TradingSymbol::ONDOUSDT,  // ONDO
            TradingSymbol::AAVEUSDT,  // AAVE
            TradingSymbol::SOLUSDT,   // Solana
        ];
        
        info!("📊 交易币种: {} 个", trading_symbols.len());

        // 加载API配置
        let user_config = load_binance_user_config()?;

        // 创建信号处理通道
        let (signal_tx, signal_rx) = mpsc::channel(1000);
        let position_manager = PositionManager::new(10000.0); // 初始余额
        
        // 创建API管理器
        let (api_manager, mut api_rx) = create_api_manager(
            user_config.api_key.clone(),
            user_config.secret_key.clone(),
        ).await?;

        // 从API管理器获取共享的BinanceFuturesApi实例
        let shared_api_client = api_manager.get_api_client();
        
        // 创建SignalManager，使用共享的API实例
        let mut signal_manager = SignalManager::new_with_client(
            signal_rx,
            position_manager,
            shared_api_client,
        );

        // 启动信号处理任务
        let signal_manager_handle = tokio::spawn(async move {
            if let Err(e) = signal_manager.process_signals().await {
                eprintln!("❌ 信号处理任务失败: {}", e);
            }
        });

        // 创建WebSocket管理器
        let (ws_manager, mut ws_rx) = create_websocket_manager().await?;

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
        for symbol in &trading_symbols {
            // 根据币种设置不同的参数（调整为1小时周期）
            let (break_period, ema_period, profit_period, atr_period, atr_multiplier) = match symbol {
                TradingSymbol::BTCUSDT => (
                    50,     // 突破周期保持不变（50小时）
                    240,    // EMA周期保持不变（240小时）
                    15,     // 止盈周期保持不变（15小时）
                    20,     // ATR周期保持不变（20小时）
                    2.5,    // ATR倍数保持不变
                ),
                TradingSymbol::ETHUSDT => (
                    45,     // 突破周期保持不变（45小时）
                    240,    // EMA周期保持不变（240小时）
                    12,     // 止盈周期保持不变（12小时）
                    20,     // ATR周期保持不变（20小时）
                    2.8,    // ATR倍数保持不变
                ),
                TradingSymbol::ONDOUSDT => (
                    30,     // 突破周期保持不变（30小时）
                    240,    // EMA周期保持不变（240小时）
                    8,      // 止盈周期保持不变（8小时）
                    20,     // ATR周期保持不变（20小时）
                    3.5,    // ATR倍数保持不变
                ),
                TradingSymbol::AAVEUSDT => (
                    32,     // 突破周期保持不变（32小时）
                    240,    // EMA周期保持不变（240小时）
                    9,      // 止盈周期保持不变（9小时）
                    20,     // ATR周期保持不变（20小时）
                    3.2,    // ATR倍数保持不变
                ),
                TradingSymbol::SOLUSDT => (
                    40,     // 突破周期保持不变（40小时）
                    240,    // EMA周期保持不变（240小时）
                    10,     // 止盈周期保持不变（10小时）
                    20,     // ATR周期保持不变（20小时）
                    3.0,    // ATR倍数保持不变
                ),
                _ => (
                    35,     // 默认突破周期保持不变（35小时）
                    240,    // 默认EMA周期保持不变（240小时）
                    10,     // 默认止盈周期保持不变（10小时）
                    20,     // 默认ATR周期保持不变（20小时）
                    3.0,    // 默认ATR倍数保持不变
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

        }

        // 启动策略管理器任务
        let strategy_manager_handle = tokio::spawn(async move {
            if let Err(e) = strategy_manager.run().await {
                error!("❌ 策略管理器运行失败: {}", e);
            }
        });

        // 获取所有币种的历史K线数据进行初始化
        info!("📈 获取历史数据初始化策略...");
        for symbol in &trading_symbols {
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
                        for kline in kline_data_list.iter() {
                            // 设置symbol字段（API数据需要手动设置）
                            let mut api_kline = kline.clone();
                            api_kline.symbol = symbol.clone();
                            
                            // 包装为统一数据类型
                            let unified_data = UnifiedKlineData::Api(api_kline);
                            
                            // 发送数据到策略管理器
                            if let Err(e) = strategy_data_tx.send(Arc::new(unified_data)).await {
                                error!("❌ 发送历史数据到策略管理器失败: {}", e);
                            }
                        }
                    }
                }
            }
        }
        info!("✅ 历史数据初始化完成");

        // 配置批量WebSocket连接 - 为所有币种
        let symbol_strings: Vec<String> = trading_symbols.iter()
            .map(|symbol| symbol.as_str().to_lowercase())
            .collect();
        
        let interval = "1h";
        
        let kline_config = KlineConfig::new_multi(
            symbol_strings.clone(),
            interval,
            WebSocketBaseConfig {
                auto_reconnect: true,
                max_retries: 5,
                retry_delay_secs: 5,
                connection_timeout_secs: 10,
                message_timeout_secs: 30,
                enable_heartbeat: true,
                heartbeat_interval_secs: 30,
                tags: vec!["q1_multi_batch".to_string()],
            },
        );

        // 启动批量WebSocket连接
        info!("🔌 建立批量WebSocket连接: {} 个币种", symbol_strings.len());
        match ws_manager.start_multi_kline(kline_config).await {
            Ok(_) => {
                info!("✅ WebSocket连接已建立");
            }
            Err(e) => {
                error!("❌ WebSocket连接失败: {}", e);
                return Err(anyhow::anyhow!("WebSocket连接失败: {}", e));
            }
        }

        // 简化的统计变量
        let mut _message_count = 0;

        // 处理实时数据
        info!("🎯 开始接收实时数据...");
        
        while let Some(message) = ws_rx.recv().await {
            _message_count += 1;

            match message {
                WebSocketMessage::Kline(kline_data) => {
                    // 发送数据到策略管理器
                    let ws_kline_data = (*kline_data).clone();
                    let unified_data = UnifiedKlineData::WebSocket(ws_kline_data);
                    if let Err(e) = strategy_data_tx.send(Arc::new(unified_data)).await {
                        error!("❌ 发送数据到策略管理器失败: {}", e);
                    }
                }
                _ => {}
            }
        }
        // 等待所有任务完成
        if let Err(e) = signal_manager_handle.await {
            eprintln!("❌ 信号处理任务异常: {}", e);
        }

        if let Err(e) = strategy_manager_handle.await {
            eprintln!("❌ 策略管理器任务异常: {:?}", e);
        }

        Ok(())
    }
}

