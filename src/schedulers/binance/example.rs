//! WsProducer 使用示例

use crate::database::connection::RedisManager;
use crate::schedulers::binance::ws_producer::WsProducer;
use anyhow::Result;

/// 演示如何使用 WsProducer
pub async fn example_usage() -> Result<()> {
    // 1. 创建 Redis 连接
    let redis_manager = RedisManager::new("redis://127.0.0.1:6379").await?;
    
    // 2. 创建 WsProducer
    let producer = WsProducer::new(redis_manager);
    
    // 3. 存储价格数据到 Stream（消息队列模式）
    println!("=== 存储价格数据到 Stream ===");
    let message_id = producer.store_price_to_stream("btcusdt", 50000.0, Some(100.5)).await?;
    println!("存储成功，消息ID: {}", message_id);
    
    // 4. 存储价格数据到 Key-Value（缓存模式）
    println!("\n=== 存储价格数据到 Key-Value ===");
    producer.store_price_to_kv("btcusdt", 50000.0).await?;
    println!("存储成功");
    
    // 5. 读取 Stream 中的数据
    println!("\n=== 读取 Stream 数据 ===");
    let prices = producer.read_price_from_stream("btcusdt", 10).await?;
    for (id, price_data) in prices {
        println!("消息ID: {}, 价格: {}, 时间: {}", id, price_data.price, price_data.timestamp);
    }
    
    // 6. 读取 Key-Value 中的数据
    println!("\n=== 读取 Key-Value 数据 ===");
    if let Some(price) = producer.read_price_from_kv("btcusdt").await? {
        println!("最新价格: {}", price);
    } else {
        println!("没有找到价格数据");
    }
    
    // 7. 存储深度数据
    println!("\n=== 存储深度数据 ===");
    let bids = vec![(49999.0, 1.5), (49998.0, 2.0)];
    let asks = vec![(50001.0, 1.0), (50002.0, 2.5)];
    let depth_id = producer.store_depth_to_stream("btcusdt", bids, asks).await?;
    println!("深度数据存储成功，消息ID: {}", depth_id);
    
    Ok(())
}

/// 演示批量存储
pub async fn batch_example() -> Result<()> {
    let redis_manager = RedisManager::new("redis://127.0.0.1:6379").await?;
    let producer = WsProducer::new(redis_manager);
    
    println!("=== 批量存储价格数据 ===");
    
    // 模拟100ms间隔的价格数据
    for i in 0..20 {
        let price = 50000.0 + (i as f64 * 10.0);
        let volume = 100.0 + (i as f64 * 5.0);
        
        producer.store_price_to_stream("btcusdt", price, Some(volume)).await?;
        producer.store_price_to_kv("btcusdt", price).await?;
        
        println!("存储价格 {}: {}", i + 1, price);
        
        // 模拟100ms间隔
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }
    
    // 读取所有数据
    println!("\n=== 读取所有价格数据 ===");
    let prices = producer.read_price_from_stream("btcusdt", 50).await?;
    println!("总共读取到 {} 条价格数据", prices.len());
    
    for (id, price_data) in prices.iter().take(5) {
        println!("消息ID: {}, 价格: {}, 时间: {}", id, price_data.price, price_data.timestamp);
    }
    
    if prices.len() > 5 {
        println!("... 还有 {} 条数据", prices.len() - 5);
    }
    
    Ok(())
} 