use rust_system::common::config::ws_config::ConfigLoader;
use rust_system::exchange_api::binance::ws_manager::{WebSocketMessage, create_websocket_manager};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== 启动 Binance K线 WebSocket 测试 ===");
    
    // 读取配置文件
    let configs = ConfigLoader::load_from_file("config.toml")?;
    let (manager, mut rx) = create_websocket_manager().await?;

    // 只启动K线连接
    for config in configs.kline {
        println!("启动K线连接: {} - 间隔: {}", config.symbol.join(", "), config.interval);
        manager.start_kline(config).await?;
    }

    // 等待连接建立
    tokio::time::sleep(Duration::from_secs(3)).await;
    
    // 检查连接状态
    let active_connections = manager.active_connections().await;
    println!("活跃连接数: {}", active_connections);
    
    let connections = manager.list_connections().await;
    for conn in connections {
        println!("连接: {} - {} - {:?}", conn.connection_id, conn.symbol, conn.data_type);
    }

    println!("\n=== 开始接收K线数据 ===");
    println!("按 Ctrl+C 停止程序\n");

    let mut message_count = 0;
    let mut last_report = std::time::Instant::now();

    // 只处理K线消息
    while let Some(msg) = rx.recv().await {
        match msg {
            WebSocketMessage::Kline(data) => {
                message_count += 1;
                println!("[K线 #{:03}] {}: 开盘={}, 最高={}, 最低={}, 收盘={}, 成交量={}, 是否关闭={}", 
                    message_count,
                    data.symbol, data.kline.open_price, data.kline.high_price, 
                    data.kline.low_price, data.kline.close_price, data.kline.base_volume,
                    data.kline.is_closed);
            }
            
            _ => {
                println!("收到其他类型消息: {:?}", msg);
            }
        }
        
        // 每10秒报告一次
        if last_report.elapsed() > Duration::from_secs(10) {
            println!("已接收 {} 条消息", message_count);
            last_report = std::time::Instant::now();
        }
    }

    Ok(())
} 