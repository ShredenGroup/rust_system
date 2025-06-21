//! WebSocket数据生产者：将Binance WebSocket数据推送到Redis Stream

use crate::database::connection::RedisManager;
use crate::database::message_queue::operations::MessageQueueOperations;
use crate::database::message_queue::entities::MarketDataMessage;
use crate::exchange_api::binance::ws_manager::WebSocketMessage;
use anyhow::Result;
use chrono::Utc;
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

/// 价格数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub symbol: String,
    pub price: f64,
    pub timestamp: i64,
    pub volume: Option<f64>,
    pub exchange: String,
}

/// 深度数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthData {
    pub symbol: String,
    pub bids: Vec<(f64, f64)>, // (price, quantity)
    pub asks: Vec<(f64, f64)>, // (price, quantity)
    pub timestamp: i64,
    pub exchange: String,
}

pub struct WsProducer {
    pub redis_manager: RedisManager,
    pub mq_ops: MessageQueueOperations,
}

impl WsProducer {
    pub fn new(redis_manager: RedisManager) -> Self {
        let mq_ops = MessageQueueOperations::new(redis_manager.clone());
        Self { redis_manager, mq_ops }
    }

    /// 推送市场数据到Redis Stream（消息队列模式）
    pub async fn push_market_data(&self, symbol: &str, price: f64, volume: f64, exchange: &str) -> Result<String> {
        let msg = MarketDataMessage {
            symbol: symbol.to_string(),
            price,
            volume,
            timestamp: Utc::now(),
            exchange: exchange.to_string(),
        };
        self.mq_ops.send_market_data(&msg).await
    }

    /// 存储价格数据到Redis Stream（保留历史，适合消息队列）
    pub async fn store_price_to_stream(&self, symbol: &str, price: f64, volume: Option<f64>) -> Result<String> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        let stream_key = format!("price_stream:{}", symbol);
        
        let price_data = PriceData {
            symbol: symbol.to_string(),
            price,
            timestamp: Utc::now().timestamp_millis(),
            volume,
            exchange: "binance".to_string(),
        };
        
        let data_json = serde_json::to_string(&price_data)?;
        
        // 使用 XADD 添加到 Stream，自动生成消息ID
        let message_id: String = redis::cmd("XADD")
            .arg(&stream_key)
            .arg("*") // 自动生成ID
            .arg("data")
            .arg(&data_json)
            .arg("timestamp")
            .arg(price_data.timestamp)
            .query_async(&mut *conn)
            .await?;
        
        // 限制Stream长度，只保留最新的100条消息
        let _: i32 = redis::cmd("XTRIM")
            .arg(&stream_key)
            .arg("MAXLEN")
            .arg("~") // 近似长度，性能更好
            .arg(100)
            .query_async(&mut *conn)
            .await?;
        
        Ok(message_id)
    }

    /// 存储价格数据到Redis Key-Value（只保留最新值）
    pub async fn store_price_to_kv(&self, symbol: &str, price: f64) -> Result<()> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        let key = format!("price:{}", symbol);
        
        // 存储最新价格
        let _: () = redis::cmd("SET")
            .arg(&key)
            .arg(price)
            .arg("EX") // 设置过期时间（可选）
            .arg(3600) // 1小时过期
            .query_async(&mut *conn)
            .await?;
        
        Ok(())
    }

    /// 存储深度数据到Redis Stream
    pub async fn store_depth_to_stream(&self, symbol: &str, bids: Vec<(f64, f64)>, asks: Vec<(f64, f64)>) -> Result<String> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        let stream_key = format!("depth_stream:{}", symbol);
        
        let depth_data = DepthData {
            symbol: symbol.to_string(),
            bids,
            asks,
            timestamp: Utc::now().timestamp_millis(),
            exchange: "binance".to_string(),
        };
        
        let data_json = serde_json::to_string(&depth_data)?;
        
        // 添加到 Stream
        let message_id: String = redis::cmd("XADD")
            .arg(&stream_key)
            .arg("*")
            .arg("data")
            .arg(&data_json)
            .arg("timestamp")
            .arg(depth_data.timestamp)
            .query_async(&mut *conn)
            .await?;
        
        // 限制Stream长度，只保留最新的100条消息
        let _: i32 = redis::cmd("XTRIM")
            .arg(&stream_key)
            .arg("MAXLEN")
            .arg("~")
            .arg(100)
            .query_async(&mut *conn)
            .await?;
        
        Ok(message_id)
    }

    /// 从Redis Stream读取价格数据（消费者模式）
    pub async fn read_price_from_stream(&self, symbol: &str, count: usize) -> Result<Vec<(String, PriceData)>> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        let stream_key = format!("price_stream:{}", symbol);
        
        // 读取Stream中的消息
        let result: Vec<(String, Vec<(String, Vec<(String, String)>)>)> = redis::cmd("XREAD")
            .arg("COUNT")
            .arg(count)
            .arg("STREAMS")
            .arg(&stream_key)
            .arg("0") // 从开始读取
            .query_async(&mut *conn)
            .await?;
        
        let mut prices = Vec::new();
        for (_, entries) in result {
            for (id, fields) in entries {
                if let Some((_, data)) = fields.iter().find(|(k, _)| k == "data") {
                    if let Ok(price_data) = serde_json::from_str::<PriceData>(data) {
                        prices.push((id, price_data));
                    }
                }
            }
        }
        
        Ok(prices)
    }

    /// 从Redis Key-Value读取最新价格
    pub async fn read_price_from_kv(&self, symbol: &str) -> Result<Option<f64>> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        let key = format!("price:{}", symbol);
        
        let result: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut *conn)
            .await?;
        
        match result {
            Some(price_str) => Ok(Some(price_str.parse::<f64>()?)),
            None => Ok(None),
        }
    }

    /// 处理WebSocket消息并存储到Redis
    pub async fn process_websocket_message(&self, message: WebSocketMessage) -> Result<()> {
        match message {
            WebSocketMessage::MarkPrice(data) => {
                // 存储标记价格
                let price = data.mark_price.parse::<f64>()?;
                self.store_price_to_stream(&data.symbol, price, None).await?;
                self.store_price_to_kv(&data.symbol, price).await?;
                println!("Stored mark price for {}: {}", data.symbol, price);
            }
            WebSocketMessage::Kline(data) => {
                // 存储K线数据
                let price = data.close_price.parse::<f64>()?;
                let volume = data.volume.parse::<f64>().ok();
                self.store_price_to_stream(&data.symbol, price, volume).await?;
                self.store_price_to_kv(&data.symbol, price).await?;
                println!("Stored kline price for {}: {}", data.symbol, price);
            }
            WebSocketMessage::PartialDepth(data) => {
                // 存储深度数据
                let bids: Vec<(f64, f64)> = data.bids.iter()
                    .map(|(price, qty)| (price.parse::<f64>().unwrap_or(0.0), qty.parse::<f64>().unwrap_or(0.0)))
                    .collect();
                let asks: Vec<(f64, f64)> = data.asks.iter()
                    .map(|(price, qty)| (price.parse::<f64>().unwrap_or(0.0), qty.parse::<f64>().unwrap_or(0.0)))
                    .collect();
                
                self.store_depth_to_stream(&data.symbol, bids, asks).await?;
                println!("Stored depth data for {}: {} bids, {} asks", data.symbol, bids.len(), asks.len());
            }
            WebSocketMessage::DiffDepth(data) => {
                // 存储深度差异数据
                let bids: Vec<(f64, f64)> = data.bids.iter()
                    .map(|(price, qty)| (price.parse::<f64>().unwrap_or(0.0), qty.parse::<f64>().unwrap_or(0.0)))
                    .collect();
                let asks: Vec<(f64, f64)> = data.asks.iter()
                    .map(|(price, qty)| (price.parse::<f64>().unwrap_or(0.0), qty.parse::<f64>().unwrap_or(0.0)))
                    .collect();
                
                self.store_depth_to_stream(&data.symbol, bids, asks).await?;
                println!("Stored diff depth data for {}: {} bids, {} asks", data.symbol, bids.len(), asks.len());
            }
        }
        
        Ok(())
    }
}

// 使用示例：
// 
// 1. 作为消息队列（Stream模式）：
//    - 生产者：store_price_to_stream() - 添加消息到队列
//    - 消费者：read_price_from_stream() - 从队列读取消息
//    - 特点：保留历史数据，支持多个消费者
//
// 2. 作为缓存（Key-Value模式）：
//    - 生产者：store_price_to_kv() - 更新最新价格
//    - 消费者：read_price_from_kv() - 读取最新价格
//    - 特点：只保留最新值，性能更好

// 后续可扩展：
// - 支持批量推送
// - 支持多种消息类型
// - 支持异步队列缓冲 