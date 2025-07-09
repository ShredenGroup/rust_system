//! 主程序入口，用于启动和集成一个简单的事件驱动策略。

// 从我们的库中导入必要的模块
use rust_system::{
    common::config::ws_config::{ConfigLoader, KlineConfig, WebSocketBaseConfig},
    exchange_api::binance::ws_manager::{WebSocketMessage, create_websocket_manager},
};
use std::time::Instant;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 启动币安 K线 WebSocket 服务");
    
    // 从配置文件读取配置
    let configs = ConfigLoader::load_from_file("./config.toml")?;
    println!("✅ 配置文件加载成功");
    
    // 创建 WebSocket 管理器
    let (ws_manager, mut message_rx) = create_websocket_manager().await?;
    
    // 使用配置文件中的 K线配置，如果没有则创建默认配置
    let kline_configs = if !configs.kline.is_empty() {
        configs.kline.clone()
    } else {
        // 创建默认配置
        let base_config = WebSocketBaseConfig {
            auto_reconnect: true,
            max_retries: 5,
            retry_delay_secs: 1,
            connection_timeout_secs: 10,
            message_timeout_secs: 5,
            enable_heartbeat: true,
            heartbeat_interval_secs: 30,
            tags: vec!["main".to_string()],
        };
        
        vec![KlineConfig::new_multi(
            vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()],
            "1m",
            base_config
        )]
    };
    
    // 启动所有 K线 WebSocket 连接
    println!("📈 启动 K线 WebSocket 连接...");
    for (i, kline_config) in kline_configs.iter().enumerate() {
        println!("   配置 {}: 交易对 {:?}, 间隔 {}", i + 1, kline_config.symbol, kline_config.interval);
        ws_manager.start_kline(kline_config.clone()).await?;
    }
    
    println!("✅ WebSocket 连接已建立");
    println!("📊 开始接收实时价格数据...");
    println!("按 Ctrl+C 停止服务");
    println!("{}", "=".repeat(60));
    
    // 统计信息
    let mut message_count = 0;
    let start_time = Instant::now();
    let mut last_prices: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    
    // 主循环：接收和处理 WebSocket 消息
    loop {
        tokio::select! {
            // 接收 WebSocket 消息
            message = message_rx.recv() => {
                match message {
                    Some(WebSocketMessage::Kline(kline_data)) => {
                        message_count += 1;
                        
                        // 提取价格信息
                        let symbol = kline_data.symbol.clone();
                        let close_price = kline_data.kline.close_price.parse::<f64>().unwrap_or(0.0);
                        let open_price = kline_data.kline.open_price.parse::<f64>().unwrap_or(0.0);
                        let high_price = kline_data.kline.high_price.parse::<f64>().unwrap_or(0.0);
                        let low_price = kline_data.kline.low_price.parse::<f64>().unwrap_or(0.0);
                        let base_volume = kline_data.kline.base_volume.parse::<f64>().unwrap_or(0.0);
                        
                        // 计算价格变化
                        let price_change = close_price - open_price;
                        let price_change_percent = if open_price > 0.0 {
                            (price_change / open_price) * 100.0
                        } else {
                            0.0
                        };
                        
                        // 获取上次价格
                        let last_price = last_prices.get(&symbol).unwrap_or(&close_price);
                        let last_change = close_price - last_price;
                        let last_change_percent = if *last_price > 0.0 {
                            (last_change / last_price) * 100.0
                        } else {
                            0.0
                        };
                        
                        // 更新最新价格
                        last_prices.insert(symbol.clone(), close_price);
                        
                        // 计算运行时间
                        let elapsed = start_time.elapsed();
                        let messages_per_second = message_count as f64 / elapsed.as_secs_f64();
                        
                        // 打印价格信息
                        println!("📊 [{}] {} | 价格: {:.2} | 开盘: {:.2} | 最高: {:.2} | 最低: {:.2} | 成交量: {:.2}",
                            kline_data.kline.start_time,
                            symbol,
                            close_price,
                            open_price,
                            high_price,
                            low_price,
                            base_volume
                        );
                        
                        println!("📈 变化: {:.2} ({:+.2}%) | 相对上次: {:.2} ({:+.2}%) | 消息/秒: {:.1}",
                            price_change,
                            price_change_percent,
                            last_change,
                            last_change_percent,
                            messages_per_second
                        );
                        
                        println!("{}", "-".repeat(60));
                    }
                    
                    Some(WebSocketMessage::MarkPrice(mark_price_data)) => {
                        println!("💰 标记价格 [{}]: ${}", 
                            mark_price_data.symbol, 
                            mark_price_data.mark_price
                        );
                    }
                    
                    Some(WebSocketMessage::PartialDepth(depth_data)) => {
                        println!("📚 深度数据 [{}]: 买单 {} 个, 卖单 {} 个", 
                            depth_data.symbol,
                            depth_data.bids.len(),
                            depth_data.asks.len()
                        );
                    }
                    
                    Some(WebSocketMessage::DiffDepth(depth_data)) => {
                        println!("🔄 深度差异 [{}]: 更新 {} 个价格档位", 
                            depth_data.symbol,
                            depth_data.bids.len() + depth_data.asks.len()
                        );
                    }
                    
                    None => {
                        println!("❌ WebSocket 连接已关闭");
                        break;
                    }
                }
            }
            
            // 处理 Ctrl+C 信号
            _ = signal::ctrl_c() => {
                println!("\n🛑 收到停止信号，正在关闭服务...");
                break;
            }
        }
    }
    
    // 清理资源
    println!("🧹 正在清理资源...");
    ws_manager.stop_all_connections().await?;
    
    // 打印统计信息
    let total_time = start_time.elapsed();
    println!("📊 服务统计:");
    println!("   总运行时间: {:.2} 秒", total_time.as_secs_f64());
    println!("   总消息数: {}", message_count);
    println!("   平均消息/秒: {:.1}", message_count as f64 / total_time.as_secs_f64());
    println!("   监控的交易对: {:?}", last_prices.keys().collect::<Vec<_>>());
    
    println!("✅ 服务已停止");
    Ok(())
}

