use crate::database::connection::RedisManager;
use crate::database::message_queue::entities::*;
use anyhow::Result;
use serde::Serialize;

pub struct MessageQueueOperations {
    redis_manager: RedisManager,
}

impl MessageQueueOperations {
    pub fn new(redis_manager: RedisManager) -> Self {
        Self { redis_manager }
    }

    // 初始化所有 Stream 和消费者组
    pub async fn initialize(&self) -> Result<()> {
        // 创建消费者组
        self.create_consumer_group(ORDER_STREAM, ORDER_PROCESSOR_GROUP).await?;
        self.create_consumer_group(MARKET_DATA_STREAM, DATA_AGGREGATOR_GROUP).await?;
        self.create_consumer_group(SIGNAL_STREAM, STRATEGY_GROUP).await?;
        self.create_consumer_group(TRADE_STREAM, TRADE_EXECUTOR_GROUP).await?;
        
        Ok(())
    }

    // 基础 Stream 操作
    async fn send_message<T: Serialize>(&self, stream_key: &str, message: &T) -> Result<String> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        let message_json = serde_json::to_string(message)?;
        
        let result: String = redis::cmd("XADD")
            .arg(stream_key)
            .arg("*") // 自动生成 ID
            .arg("data")
            .arg(&message_json)
            .arg("timestamp")
            .arg(chrono::Utc::now().timestamp_millis())
            .query_async(&mut *conn)
            .await?;
        
        Ok(result)
    }

    async fn create_consumer_group(&self, stream_key: &str, group_name: &str) -> Result<()> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        
        // 尝试创建消费者组，如果已存在则忽略错误
        let _: Result<(), _> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(stream_key)
            .arg(group_name)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut *conn)
            .await;
        
        Ok(())
    }

    async fn read_from_group(&self, stream_key: &str, group_name: &str, consumer_name: &str, count: usize) -> Result<Vec<(String, String)>> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        
        let result: Vec<(String, Vec<(String, Vec<(String, String)>)>)> = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(group_name)
            .arg(consumer_name)
            .arg("COUNT")
            .arg(count)
            .arg("STREAMS")
            .arg(stream_key)
            .arg(">") // 只读取新消息
            .query_async(&mut *conn)
            .await?;
        
        let mut messages = Vec::new();
        for (_, entries) in result {
            for (id, fields) in entries {
                if let Some((_, data)) = fields.iter().find(|(k, _)| k == "data") {
                    messages.push((id, data.clone()));
                }
            }
        }
        
        Ok(messages)
    }

    async fn ack_message(&self, stream_key: &str, group_name: &str, message_id: &str) -> Result<()> {
        let mut conn = self.redis_manager.get_connection_mut().await;
        
        let _: i32 = redis::cmd("XACK")
            .arg(stream_key)
            .arg(group_name)
            .arg(message_id)
            .query_async(&mut *conn)
            .await?;
        
        Ok(())
    }

    // 订单相关操作
    pub async fn send_order(&self, order: &OrderMessage) -> Result<String> {
        self.send_message(ORDER_STREAM, order).await
    }

    pub async fn read_orders(&self, consumer_name: &str, count: usize) -> Result<Vec<(String, OrderMessage)>> {
        let messages = self.read_from_group(ORDER_STREAM, ORDER_PROCESSOR_GROUP, consumer_name, count).await?;
        
        let mut orders = Vec::new();
        for (id, data) in messages {
            let order: OrderMessage = serde_json::from_str(&data)?;
            orders.push((id, order));
        }
        
        Ok(orders)
    }

    pub async fn ack_order(&self, message_id: &str) -> Result<()> {
        self.ack_message(ORDER_STREAM, ORDER_PROCESSOR_GROUP, message_id).await
    }

    // 市场数据相关操作
    pub async fn send_market_data(&self, data: &MarketDataMessage) -> Result<String> {
        self.send_message(MARKET_DATA_STREAM, data).await
    }

    pub async fn read_market_data(&self, consumer_name: &str, count: usize) -> Result<Vec<(String, MarketDataMessage)>> {
        let messages = self.read_from_group(MARKET_DATA_STREAM, DATA_AGGREGATOR_GROUP, consumer_name, count).await?;
        
        let mut data = Vec::new();
        for (id, json_data) in messages {
            let market_data: MarketDataMessage = serde_json::from_str(&json_data)?;
            data.push((id, market_data));
        }
        
        Ok(data)
    }

    pub async fn ack_market_data(&self, message_id: &str) -> Result<()> {
        self.ack_message(MARKET_DATA_STREAM, DATA_AGGREGATOR_GROUP, message_id).await
    }

    // 信号相关操作
    pub async fn send_signal(&self, signal: &SignalMessage) -> Result<String> {
        self.send_message(SIGNAL_STREAM, signal).await
    }

    pub async fn read_signals(&self, consumer_name: &str, count: usize) -> Result<Vec<(String, SignalMessage)>> {
        let messages = self.read_from_group(SIGNAL_STREAM, STRATEGY_GROUP, consumer_name, count).await?;
        
        let mut signals = Vec::new();
        for (id, data) in messages {
            let signal: SignalMessage = serde_json::from_str(&data)?;
            signals.push((id, signal));
        }
        
        Ok(signals)
    }

    pub async fn ack_signal(&self, message_id: &str) -> Result<()> {
        self.ack_message(SIGNAL_STREAM, STRATEGY_GROUP, message_id).await
    }

    // 交易相关操作
    pub async fn send_trade(&self, trade: &TradeMessage) -> Result<String> {
        self.send_message(TRADE_STREAM, trade).await
    }

    pub async fn read_trades(&self, consumer_name: &str, count: usize) -> Result<Vec<(String, TradeMessage)>> {
        let messages = self.read_from_group(TRADE_STREAM, TRADE_EXECUTOR_GROUP, consumer_name, count).await?;
        
        let mut trades = Vec::new();
        for (id, data) in messages {
            let trade: TradeMessage = serde_json::from_str(&data)?;
            trades.push((id, trade));
        }
        
        Ok(trades)
    }

    pub async fn ack_trade(&self, message_id: &str) -> Result<()> {
        self.ack_message(TRADE_STREAM, TRADE_EXECUTOR_GROUP, message_id).await
    }
} 