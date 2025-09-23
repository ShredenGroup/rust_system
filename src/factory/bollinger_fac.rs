use crate::{
    common::{
        config::ws_config::{KlineConfig, WebSocketBaseConfig},
        config::user_config::load_binance_user_config,
        ts::Strategy,
        consts::TURBO_USDT_SYMBOL,
    },
    models::Side,
    exchange_api::binance::{
        ws_manager::{create_websocket_manager, WebSocketMessage},
        api_manager::{create_api_manager, ApiMessage},
    },
    strategy::bollinger::BollingerStrategy,
    order::filter_manager::{SignalManager, PositionManager},
};
use ta::Next;
use anyhow::Result;
use chrono::{DateTime, Local};
use std::time::Instant;
use tracing::{info, debug, error};
use tracing_subscriber::EnvFilter;
use std::fs;
use std::path::Path;
use tokio::sync::mpsc;

/// 布林带策略工厂
pub struct BollingerFactory;

impl BollingerFactory {
    /// 设置日志系统
    pub fn setup_logging() -> Result<()> {
        // 创建logs目录
        let log_dir = "logs";
        if !Path::new(log_dir).exists() {
            fs::create_dir(log_dir)?;
        }

        // 配置日志过滤器
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,bollinger_factory=debug"));

        // 配置文件输出
        let file_appender = tracing_appender::rolling::daily(log_dir, "bollinger_factory.log");

        // 初始化日志系统
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(file_appender)
            .init();

        info!("🚀 布林带策略工厂启动");
        info!("📁 日志文件保存在: {}", log_dir);
        
        Ok(())
    }

    /// 运行布林带策略
    pub async fn run_bollinger_strategy() -> Result<()> {
        info!("🚀 启动布林带策略");
        info!("{}", "=".repeat(60));

        // 加载API配置
        let user_config = load_binance_user_config()?;
        info!("✅ 已加载用户配置");

        // 创建信号处理通道
        let (signal_tx, signal_rx) = mpsc::channel(1000);
        let position_manager = PositionManager::new(10000.0); // 初始余额
        
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
            position_manager,
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

        // 创建布林带策略实例
        let mut bollinger_strategy = BollingerStrategy::new(20, 2.0)?;
        info!("✅ 布林带策略初始化完成 (参数: period=20, std_dev=2.0)");

        // 获取历史K线数据进行初始化
        info!("📊 获取历史K线数据进行初始化...");
        api_manager.get_history_klines(
            TURBO_USDT_SYMBOL.to_string(),
            "1h".to_string(),
            None,
            None,
            Some("21".to_string()),  // 获取足够的K线用于初始化
        ).await?;

        // 等待并处理历史数据
        if let Some(message) = api_rx.recv().await {
            match message {
                ApiMessage::Kline(kline_data_list) => {
                    info!("📈 收到历史K线数据: {} 根", kline_data_list.len());
                    for kline in kline_data_list.iter() {
                        // 构造 KlineData 结构
                        let kline_data = crate::dto::binance::websocket::KlineData {
                            event_type: "kline".to_string(),
                            event_time: kline.open_time,
                            symbol: crate::common::TradingSymbol::from_string(TURBO_USDT_SYMBOL.to_string()),
                            kline: crate::dto::binance::websocket::KlineInfo {
                                start_time: kline.open_time,
                                close_time: kline.close_time,
                                symbol: crate::common::TradingSymbol::from_string(TURBO_USDT_SYMBOL.to_string()),
                                interval: "1h".to_string(), // 使用配置的间隔
                                first_trade_id: 0,
                                last_trade_id: 0,
                                open_price: kline.open,
                                close_price: kline.close,
                                high_price: kline.high,
                                low_price: kline.low,
                                base_volume: kline.volume,
                                trade_count: kline.trades_count,
                                is_closed: true, // 历史数据都是已完成的
                                quote_volume: kline.quote_volume,
                                taker_buy_base_volume: kline.taker_buy_volume,
                                taker_buy_quote_volume: kline.taker_buy_quote_volume,
                                ignore: "".to_string(),
                            },
                        };
                        
                        if let Some(signal) = bollinger_strategy.on_kline_update(&kline_data) {
                            info!("⚡ 历史数据产生信号:");
                            let timestamp = DateTime::from_timestamp_millis(kline.open_time).unwrap();
                            info!("   时间: {}", timestamp.format("%Y-%m-%d %H:%M:%S"));
                            info!("   价格: {}", kline.close);
                            info!("   信号: {:?}", signal);
                            
                            // 发送历史信号到信号管理器（仅记录，不执行交易）
                            debug!("📤 发送历史信号到信号管理器（仅记录）");
                        }
                        // 打印布林带值
                        let bb_output = bollinger_strategy.bollinger.next(&kline_data);
                        let atr_value = bollinger_strategy.atr.next(&kline_data);
                        debug!("   布林带值: {:.2}/{:.2}/{:.2}, ATR: {:.2}", 
                            bb_output.upper, bb_output.average, bb_output.lower, atr_value);
                    }
                    info!("✅ 历史数据初始化完成");
                }
            }
        }

        info!("{}", "=".repeat(60));
        info!("🔄 开始实时数据处理");

        // 配置WebSocket连接
        let symbol = TURBO_USDT_SYMBOL.to_lowercase();
        let interval = "1h";
        let kline_config = KlineConfig::new(
            &symbol,
            interval,
            WebSocketBaseConfig {
                auto_reconnect: true,
                max_retries: 5,
                retry_delay_secs: 5,
                connection_timeout_secs: 10,
                message_timeout_secs: 30,
                enable_heartbeat: true,
                heartbeat_interval_secs: 30,
                tags: vec!["bollinger_factory".to_string()],
            },
        );

        // 启动WebSocket连接
        info!("🔌 尝试建立WebSocket连接: {}/{}", symbol, interval);
        match ws_manager.start_kline(kline_config).await {
            Ok(_) => {
                info!("✅ WebSocket连接已建立");
            }
            Err(e) => {
                error!("❌ WebSocket连接失败: {}", e);
                return Err(anyhow::anyhow!("WebSocket连接失败: {}", e));
            }
        }

        let mut message_count = 0;
        let mut kline_count = 0;
        let mut closed_kline_count = 0;
        let mut signal_count = 0;
        let mut total_latency = 0.0f64;
        let mut min_latency = f64::MAX;
        let mut max_latency: f64 = 0.0;

        // 处理实时数据
        let start_time = Instant::now();
        while let Some(message) = ws_rx.recv().await {
            let ws_received_time = Instant::now();
            message_count += 1;

            if message_count % 10 == 0 {
                info!("📊 统计信息: 总消息数={}, K线数量={}, 已完成K线={}, 信号数量={}", 
                    message_count, kline_count, closed_kline_count, signal_count);
            }

            match message {
                WebSocketMessage::Kline(kline_data) => {
                    kline_count += 1;
                    let kline_info = &kline_data.kline;
                    
                    debug!("📈 收到K线数据: 交易对={}, 间隔={}, 开盘价={:.2}, 最高价={:.2}, 最低价={:.2}, 收盘价={:.2}, 是否完成={}", 
                        kline_info.symbol, kline_info.interval, kline_info.open_price, 
                        kline_info.high_price, kline_info.low_price, kline_info.close_price, 
                        kline_info.is_closed);

                    let strategy_start_time = Instant::now();
                    if let Some(signal) = bollinger_strategy.on_kline_update(kline_data.as_ref()) {
                        let strategy_latency = strategy_start_time.elapsed().as_secs_f64() * 1000.0;
                        signal_count += 1;

                        // 更新延迟统计
                        total_latency += strategy_latency;
                        min_latency = min_latency.min(strategy_latency);
                        max_latency = max_latency.max(strategy_latency);

                        // 记录交易信号
                        info!("⚡ 新交易信号生成:");
                        let now = Local::now();
                        info!("   时间: {}", now.format("%Y-%m-%d %H:%M:%S"));
                        info!("   价格: {:.2}", kline_info.close_price);
                        info!("   信号: {:?}", signal);
                        info!("   计算延迟: {:.3} ms", strategy_latency);
                        
                        // 判断信号类型并记录详细信息
                        let is_close_signal = signal.side == Side::Buy && bollinger_strategy.current_signal == 0 
                            || signal.side == Side::Sell && bollinger_strategy.current_signal == 0;
                        
                        if is_close_signal {
                            info!("🔄 平仓信号 - 交易对: {}, 数量: {}, 价格: {:.2}", 
                                signal.symbol, signal.quantity, signal.latest_price);
                            info!("   平仓类型: {}", 
                                if signal.side == Side::Buy { "买入平空" } else { "卖出平多" });
                        } else {
                            match signal.side {
                                Side::Buy => {
                                    info!("🟢 开仓买入信号 - 交易对: {}, 数量: {}, 价格: {:.2}", 
                                        signal.symbol, signal.quantity, signal.latest_price);
                                }
                                Side::Sell => {
                                    info!("🔴 开仓卖出信号 - 交易对: {}, 数量: {}, 价格: {:.2}", 
                                        signal.symbol, signal.quantity, signal.latest_price);
                                }
                            }
                        }
                        
                        // 记录当时的布林带值
                        info!("📊 信号生成时的布林带值:");
                        info!("   上轨: {:.2} (距当前价: {:.2}%)", 
                            bollinger_strategy.last_upper,
                            (bollinger_strategy.last_upper / kline_info.close_price - 1.0) * 100.0);
                        info!("   中轨: {:.2} (距当前价: {:.2}%)", 
                            bollinger_strategy.last_middle,
                            (bollinger_strategy.last_middle / kline_info.close_price - 1.0) * 100.0);
                        info!("   下轨: {:.2} (距当前价: {:.2}%)", 
                            bollinger_strategy.last_lower,
                            (bollinger_strategy.last_lower / kline_info.close_price - 1.0) * 100.0);
                        info!("   ATR: {:.2}", bollinger_strategy.last_atr);
                        info!("   当前持仓状态: {}", 
                            match bollinger_strategy.current_signal {
                                0 => "无持仓",
                                1 => "多头",
                                2 => "空头",
                                _ => "未知"
                            });
                        
                        // 🚀 发送信号到SignalManager进行交易执行
                        info!("📤 发送交易信号到信号管理器...");
                        match signal_tx.send(signal).await {
                            Ok(_) => {
                                info!("✅ 信号发送成功，等待执行");
                            }
                            Err(e) => {
                                eprintln!("❌ 信号发送失败: {}", e);
                            }
                        }
                    }

                    // 打印布林带值
                    debug!("📊 布林带指标: 上轨={:.2}, 中轨={:.2}, 下轨={:.2}, ATR={:.2}", 
                        bollinger_strategy.last_upper, bollinger_strategy.last_middle, 
                        bollinger_strategy.last_lower, bollinger_strategy.last_atr);

                    // 如果K线已完成，更新计数
                    if kline_info.is_closed {
                        closed_kline_count += 1;
                    }

                    let total_processing_time = ws_received_time.elapsed().as_secs_f64() * 1000.0;
                    debug!("   总处理延迟: {:.3} ms", total_processing_time);
                }
                _ => {}
            }

            // 更新性能统计
            if message_count % 1000 == 0 {
                let elapsed = start_time.elapsed().as_secs_f64();
                info!("📊 性能统计: 处理消息数={}, 信号生成数={}, 运行时间={:.2}秒, 消息处理率={:.2}条/秒, 信号生成率={:.2}个/秒", 
                    message_count, signal_count, elapsed, 
                    message_count as f64 / elapsed, signal_count as f64 / elapsed);
            }
        }

        info!("🏁 布林带策略结束");
        info!("最终统计: 总消息数={}, K线数量={}, 已完成K线={}, 信号数量={}", 
            message_count, kline_count, closed_kline_count, signal_count);
        if signal_count > 0 {
            let avg_latency = total_latency / signal_count as f64;
            info!("延迟统计: 最小={:.3}ms, 最大={:.3}ms, 平均={:.3}ms", 
                min_latency, max_latency, avg_latency);
        }

        // 等待信号处理任务完成
        info!("⏳ 等待信号处理任务完成...");
        if let Err(e) = signal_manager_handle.await {
            eprintln!("❌ 信号处理任务异常: {}", e);
        }

        Ok(())
    }
}
