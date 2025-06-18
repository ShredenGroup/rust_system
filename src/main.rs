use rust_system::database::connection::RedisManager;
use rust_system::database::message_queue::entities::OrderMessage;
use rust_system::database::message_queue::operations::MessageQueueOperations;
use rust_system::exchange_api::binance::ws_manager::{WebSocketConfig, create_websocket_manager};
use rust_system::common::config::{init_config, get_config};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Rust Trading System...");

    // 初始化配置
    println!("📋 Loading configuration...");
    init_config()?;
    let config = get_config();
    println!("✅ Configuration loaded successfully");
    println!("   Environment: {}", config.environment);
    println!("   Database: {}", config.database.url);
    println!("   Redis: {}", config.redis.url);
    println!("   Exchanges: {}", config.exchanges.len());
    println!("   WebSocket symbols: {:?}", config.websocket.symbols);
    println!("   Enabled strategies: {:?}", config.get_enabled_strategies());

    // 测试 WebSocket 功能
    println!("🔌 Testing WebSocket connections...");
    test_websocket_with_config().await?;

    // 测试消息队列功能
    println!("📨 Testing Message Queue...");
    test_message_queue_with_config().await?;

    println!("🎉 All tests completed successfully!");
    Ok(())
}

async fn test_websocket_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config();
    let (manager, mut rx) = create_websocket_manager().await?;
    
    // 使用配置文件中的设置
    let ws_config = WebSocketConfig {
        symbol: config.websocket.symbols[0].clone(),
        interval: config.websocket.interval.clone(),
        auto_reconnect: config.websocket.auto_reconnect,
        max_retries: config.websocket.max_retries as usize,
        retry_delay: std::time::Duration::from_secs(config.websocket.retry_delay as u64),
    };
    
    // 启动连接
    manager.start_connection(ws_config).await?;
    println!("✅ WebSocket connection started for {}", config.websocket.symbols[0]);
    
    // 接收消息
    let mut message_count = 0;
    let max_messages = 3;
    
    while let Some(data) = rx.recv().await {
        println!("📊 Received mark price: {} = ${}", data.symbol, data.mark_price);
        message_count += 1;
        
        if message_count >= max_messages {
            break;
        }
    }
    
    // 停止连接
    manager.stop_all_connections().await?;
    println!("✅ WebSocket connection stopped");
    
    Ok(())
}

async fn test_message_queue_with_config() -> Result<(), Box<dyn std::error::Error>> {
    let config = get_config();
    
    // 初始化 Redis 连接
    let redis_manager = RedisManager::new(&config.redis.url).await?;
    println!("✅ Redis connection established");

    // 初始化消息队列操作
    let mq_ops = MessageQueueOperations::new(redis_manager);
    mq_ops.initialize().await?;
    println!("✅ Message queue initialized");

    // 测试发送订单消息
    let order = OrderMessage {
        order_id: "order_001".to_string(),
        symbol: config.websocket.symbols[0].clone(),
        side: "BUY".to_string(),
        quantity: config.trading.default_quantity,
        price: Some(45000.0),
        order_type: "LIMIT".to_string(),
        timestamp: Utc::now(),
        user_id: Some("user_001".to_string()),
    };

    let message_id = mq_ops.send_order(&order).await?;
    println!("✅ Order sent with ID: {}", message_id);

    // 测试读取订单消息
    let orders = mq_ops.read_orders("test_consumer", 10).await?;
    println!("✅ Read {} orders", orders.len());

    for (id, order) in orders {
        println!("📋 Order ID: {}, Symbol: {}, Side: {}", id, order.symbol, order.side);
        // 确认消息已处理
        mq_ops.ack_order(&id).await?;
    }

    Ok(())
}
