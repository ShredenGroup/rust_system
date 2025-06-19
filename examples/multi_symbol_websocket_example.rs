use std::time::Duration;
use anyhow::Result;
use tokio::sync::mpsc;

// 从项目中导入 WebSocket 管理器相关模块
use rust_system::exchange_api::binance::ws_manager::{
    WebSocketManager, 
    MarkPriceConfig, 
    KlineConfig, 
    DepthConfig,
    TradeConfig,
    TickerConfig,
    WebSocketMessage,
    create_websocket_manager
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 启动多交易对 WebSocket 示例...");
    
    // 创建 WebSocket 管理器
    let (manager, mut rx) = create_websocket_manager().await?;
    
    // 示例 1: 单个交易对配置
    println!("\n📊 示例 1: 单个交易对配置");
    let single_mark_price = MarkPriceConfig::new("btcusdt", "1s")
        .base
        .with_auto_reconnect(true)
        .with_max_retries(5)
        .with_tag("single");
    println!("单个交易对配置: {:?}", single_mark_price);
    
    // 示例 2: 多个交易对配置
    println!("\n📈 示例 2: 多个交易对配置");
    let multi_kline = KlineConfig::new_multi(
        vec!["btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string()], 
        "1m"
    )
    .base
    .with_auto_reconnect(true)
    .with_max_retries(10)
    .with_retry_delay(Duration::from_secs(3))
    .with_tags(vec!["multi".to_string(), "popular".to_string()]);
    println!("多个交易对配置: {:?}", multi_kline);
    
    // 示例 3: 链式添加交易对
    println!("\n🔍 示例 3: 链式添加交易对");
    let chain_depth = DepthConfig::new("btcusdt", "250ms")
        .with_symbol("ethusdt")
        .with_symbol("bnbusdt")
        .with_symbols(vec!["adausdt".to_string(), "dogeusdt".to_string()])
        .base
        .with_auto_reconnect(true)
        .with_max_retries(20)
        .with_retry_delay(Duration::from_millis(100))
        .with_tags(vec!["chain".to_string(), "depth".to_string()]);
    println!("链式添加交易对配置: {:?}", chain_depth);
    
    // 示例 4: 高频交易配置
    println!("\n⚡ 示例 4: 高频交易配置");
    let hft_config = MarkPriceConfig::new_multi(
        vec!["btcusdt".to_string(), "ethusdt".to_string()], 
        "1s"
    )
    .base
    .with_auto_reconnect(true)
    .with_max_retries(50)
    .with_retry_delay(Duration::from_millis(50))
    .with_connection_timeout(Duration::from_secs(2))
    .with_message_timeout(Duration::from_secs(5))
    .with_heartbeat(true, Duration::from_secs(5))
    .with_tags(vec!["hft".to_string(), "latency_critical".to_string()]);
    println!("高频交易配置: {:?}", hft_config);
    
    // 示例 5: 投资组合监控配置
    println!("\n💼 示例 5: 投资组合监控配置");
    let portfolio_symbols = vec![
        "btcusdt".to_string(), "ethusdt".to_string(), "bnbusdt".to_string(),
        "adausdt".to_string(), "dogeusdt".to_string(), "solusdt".to_string(),
        "dotusdt".to_string(), "linkusdt".to_string(), "maticusdt".to_string()
    ];
    let portfolio_config = TickerConfig::new_multi(portfolio_symbols)
        .base
        .with_auto_reconnect(true)
        .with_max_retries(10)
        .with_retry_delay(Duration::from_secs(10))
        .with_heartbeat(true, Duration::from_secs(60))
        .with_tags(vec!["portfolio".to_string(), "monitoring".to_string()]);
    println!("投资组合监控配置: {:?}", portfolio_config);
    
    // 启动连接
    println!("\n🔌 启动 WebSocket 连接...");
    
    // 启动单个交易对连接
    manager.start_mark_price(single_mark_price).await?;
    println!("✅ 单个交易对连接已启动");
    
    // 启动多个交易对连接
    manager.start_kline(multi_kline).await?;
    println!("✅ 多个交易对K线连接已启动");
    
    // 启动链式添加的交易对连接
    manager.start_depth(chain_depth).await?;
    println!("✅ 链式添加交易对深度连接已启动");
    
    // 启动高频交易连接
    manager.start_mark_price(hft_config).await?;
    println!("✅ 高频交易连接已启动");
    
    // 启动投资组合监控连接
    manager.start_depth(portfolio_config).await?;
    println!("✅ 投资组合监控连接已启动");
    
    // 显示连接状态
    println!("\n📋 连接状态:");
    let connections = manager.list_connections().await;
    for conn in connections {
        println!("  - {} ({}) - {:?}", conn.connection_id, conn.symbols.join(","), conn.status);
        println!("    标签: {:?}", conn.tags);
    }
    
    // 按交易对查询连接
    println!("\n🔍 按交易对查询连接:");
    let btc_connections = manager.get_connections_by_symbol("btcusdt").await;
    println!("  BTC相关连接: {}", btc_connections.len());
    
    let eth_connections = manager.get_connections_by_symbol("ethusdt").await;
    println!("  ETH相关连接: {}", eth_connections.len());
    
    // 按标签查询连接
    println!("\n🏷️  按标签查询连接:");
    let hft_connections = manager.get_connections_by_tag("hft").await;
    println!("  高频交易连接: {}", hft_connections.len());
    
    let portfolio_connections = manager.get_connections_by_tag("portfolio").await;
    println!("  投资组合连接: {}", portfolio_connections.len());
    
    // 消息处理任务
    let message_handle = tokio::spawn(async move {
        let mut message_count = 0;
        let mut symbol_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let start_time = std::time::Instant::now();
        
        while let Some(message) = rx.recv().await {
            message_count += 1;
            
            match message {
                WebSocketMessage::MarkPrice(data) => {
                    let count = symbol_counts.entry(data.symbol.clone()).or_insert(0);
                    *count += 1;
                    
                    if message_count % 100 == 0 {
                        println!("📊 标记价格: {} = ${:.2} (总计: {})", 
                                data.symbol, data.mark_price, count);
                    }
                },
                WebSocketMessage::Kline(data) => {
                    let count = symbol_counts.entry(data.symbol.clone()).or_insert(0);
                    *count += 1;
                    
                    if message_count % 50 == 0 {
                        println!("📈 K线数据: {} {} 开盘:${:.2} 收盘:${:.2} (总计: {})", 
                                data.symbol, data.interval, data.open, data.close, count);
                    }
                },
                WebSocketMessage::Depth(data) => {
                    let count = symbol_counts.entry(data.symbol.clone()).or_insert(0);
                    *count += 1;
                    
                    if message_count % 200 == 0 {
                        println!("🔍 深度更新: {} 买一:${:.2} 卖一:${:.2} (总计: {})", 
                                data.symbol, data.bids[0].0, data.asks[0].0, count);
                    }
                },
            }
            
            // 显示统计信息
            if message_count % 1000 == 0 {
                let elapsed = start_time.elapsed();
                let rate = message_count as f64 / elapsed.as_secs_f64();
                println!("📊 消息统计: {} 条消息, 速率: {:.1} 消息/秒", message_count, rate);
                
                // 显示各交易对的消息数量
                println!("📈 各交易对消息数量:");
                for (symbol, count) in &symbol_counts {
                    println!("  {}: {} 条", symbol, count);
                }
            }
        }
    });
    
    // 监控任务
    let manager_clone = manager.clone();
    let monitor_handle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(10)).await;
            
            let active_count = manager_clone.active_connections().await;
            let connections = manager_clone.list_connections().await;
            
            println!("\n📊 监控报告:");
            println!("  活跃连接数: {}", active_count);
            
            // 统计各交易对的使用情况
            let mut symbol_usage: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for conn in connections {
                let uptime = conn.created_at.elapsed();
                println!("  - {}: 运行时间 {:?}, 状态: {:?}", 
                        conn.connection_id, uptime, conn.status);
                
                // 统计交易对使用次数
                for symbol in conn.symbols {
                    let count = symbol_usage.entry(symbol).or_insert(0);
                    *count += 1;
                }
            }
            
            // 显示交易对使用统计
            if !symbol_usage.is_empty() {
                println!("📈 交易对使用统计:");
                for (symbol, count) in symbol_usage {
                    println!("  {}: 在 {} 个连接中使用", symbol, count);
                }
            }
        }
    });
    
    // 运行一段时间
    println!("\n⏱️  运行 60 秒...");
    tokio::time::sleep(Duration::from_secs(60)).await;
    
    // 停止所有连接
    println!("\n🛑 停止所有连接...");
    manager.stop_all_connections().await?;
    
    // 取消任务
    message_handle.abort();
    monitor_handle.abort();
    
    println!("✅ 多交易对示例完成!");
    
    Ok(())
}

// 高级使用示例：动态添加交易对
pub async fn dynamic_symbol_management() -> Result<()> {
    println!("🚀 动态交易对管理示例...");
    
    let (manager, mut rx) = create_websocket_manager().await?;
    
    // 创建初始配置
    let mut dynamic_config = MarkPriceConfig::new("btcusdt", "1s")
        .base
        .with_auto_reconnect(true)
        .with_max_retries(5)
        .with_tag("dynamic");
    
    // 动态添加交易对
    dynamic_config = dynamic_config
        .with_symbol("ethusdt")
        .with_symbol("bnbusdt");
    
    println!("初始配置: {:?}", dynamic_config);
    
    // 启动连接
    manager.start_mark_price(dynamic_config).await?;
    
    // 模拟动态添加更多交易对
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // 创建新的配置来添加更多交易对
    let additional_config = MarkPriceConfig::new_multi(
        vec!["adausdt".to_string(), "dogeusdt".to_string(), "solusdt".to_string()], 
        "1s"
    )
    .base
    .with_auto_reconnect(true)
    .with_max_retries(5)
    .with_tag("dynamic");
    
    manager.start_mark_price(additional_config).await?;
    println!("添加了更多交易对");
    
    // 运行一段时间
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    // 停止所有连接
    manager.stop_all_connections().await?;
    
    Ok(())
}

// 性能测试：大量交易对配置
pub async fn performance_test() -> Result<()> {
    println!("🚀 性能测试：大量交易对配置...");
    
    let (manager, _rx) = create_websocket_manager().await?;
    
    // 创建大量交易对
    let symbols: Vec<String> = (0..100)
        .map(|i| format!("symbol{}usdt", i))
        .collect();
    
    // 分批创建配置
    let batch_size = 10;
    for i in 0..(symbols.len() / batch_size) {
        let start = i * batch_size;
        let end = (i + 1) * batch_size;
        let batch_symbols = symbols[start..end].to_vec();
        
        let config = MarkPriceConfig::new_multi(batch_symbols, "1s")
            .base
            .with_auto_reconnect(true)
            .with_max_retries(3)
            .with_tag("performance_test");
        
        manager.start_mark_price(config).await?;
        
        if i % 5 == 0 {
            println!("已创建 {} 批配置", i + 1);
        }
    }
    
    println!("总共创建了 {} 个交易对的配置", symbols.len());
    
    // 运行一段时间
    tokio::time::sleep(Duration::from_secs(10)).await;
    
    // 停止所有连接
    manager.stop_all_connections().await?;
    
    Ok(())
} 