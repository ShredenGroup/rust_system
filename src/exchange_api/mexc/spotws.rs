use anyhow::Result;
use futures::{StreamExt, SinkExt}; // 添加 SinkExt
use serde_json;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage}; // 重命名
use url::Url;

// 导入 MEXC 的 protobuf 结构体和 Message trait
use crate::dto::mexc::websocket::PushDataV3ApiWrapper;
use prost::Message as ProstMessage; // 重命名

#[derive(Debug, Clone)]
pub struct MexcWebSocket {
    base_url: String,
}

impl MexcWebSocket {
    pub fn new() -> Self {
        Self {
            // 使用和 Python 脚本一样的地址
            base_url: "wss://wbs-api.mexc.com/ws".to_string(),
        }
    }



    /// 订阅 K线数据
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "BTCUSDT" (必须大写)
    /// * `interval` - K线间隔，如 "Min1", "Min5", "Min15", "Min30", "Min60", "Hour4", "Hour8", "Day1", "Week1", "Month1"
    /// * `tx` - 消息发送通道
    pub async fn subscribe_kline(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<String>, // 暂时用String，后续解析protobuf
    ) -> Result<()> {
        let ws_url = self.base_url.clone();
        println!("Connecting to MEXC WebSocket: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ MEXC WebSocket connected successfully");

        let (mut write, mut read) = ws_stream.split();

        // 发送订阅请求
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIPTION",
            "params": [
                format!("spot@public.kline.v3.api.pb@{}@{}", symbol.to_uppercase(), interval)
            ]
        });

        let subscribe_text = subscribe_msg.to_string();
        println!("📤 发送订阅请求: {}", subscribe_text);
        
        let msg = WsMessage::Text(subscribe_text);
        write.send(msg).await?;

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                WsMessage::Text(text) => {
                    println!("📥 收到文本消息: {}", text);
                    
                    // 检查是否是订阅响应
                    if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(method) = response.get("method") {
                            if method == "SUBSCRIPTION" {
                                println!("✅ 订阅成功: {}", text);
                            }
                        }
                    }
                }
                WsMessage::Binary(data) => {
                    println!(" 收到二进制数据(protobuf)，长度: {}", data.len());
                    
                    // 尝试解析 MEXC 官方 protobuf 结构
                    match PushDataV3ApiWrapper::decode(&*data) {
                        Ok(wrapper) => {
                            if let Some(kline) = wrapper.extract_kline_data() {
                                println!("✅ 解析成功: {} | 开盘: {} | 收盘: {} | 最高: {} | 最低: {} | 成交量: {}", 
                                    wrapper.channel, kline.opening_price, kline.closing_price, 
                                    kline.highest_price, kline.lowest_price, kline.volume);
                                
                                // 转换为 JSON 格式发送到通道
                                let symbol_name = wrapper.symbol.clone().unwrap_or_else(|| symbol.to_uppercase());
                                let json_data = serde_json::json!({
                                    "symbol": symbol_name,
                                    "interval": kline.interval,
                                    "open": kline.opening_price.parse::<f64>().unwrap_or(0.0),
                                    "close": kline.closing_price.parse::<f64>().unwrap_or(0.0),
                                    "high": kline.highest_price.parse::<f64>().unwrap_or(0.0),
                                    "low": kline.lowest_price.parse::<f64>().unwrap_or(0.0),
                                    "volume": kline.volume.parse::<f64>().unwrap_or(0.0),
                                    "amount": kline.amount.parse::<f64>().unwrap_or(0.0),
                                    "window_start": kline.window_start,
                                    "window_end": kline.window_end,
                                    "timestamp": chrono::Utc::now().timestamp()
                                });
                                
                                if let Err(e) = tx.send(json_data.to_string()) {
                                    eprintln!("Failed to send parsed message: {}", e);
                                    break;
                                }
                            } else {
                                println!("❌ 包装器中没有K线数据");
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ Protobuf 解析失败: {}", e);
                            // 如果解析失败，发送原始十六进制数据用于调试
                            let hex_data = hex::encode(&data);
                            if let Err(e) = tx.send(format!("PROTOBUF_PARSE_ERROR:{}", hex_data)) {
                                eprintln!("Failed to send error message: {}", e);
                                break;
                            }
                        }
                    }
                }
                WsMessage::Close(_) => {
                    println!("❌ MEXC WebSocket connection closed");
                    break;
                }
                WsMessage::Ping(data) => {
                    println!(" 收到 Ping，发送 Pong 响应");
                    let pong_msg = WsMessage::Pong(data);
                    if let Err(e) = write.send(pong_msg).await {
                        eprintln!("Failed to send pong: {}", e);
                        break;
                    }
                }
                WsMessage::Pong(_) => {
                    println!("🏓 收到 Pong");
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 订阅成交数据
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "BTCUSDT" (必须大写)
    /// * `interval` - 推送间隔，如 "100ms", "10ms"
    /// * `tx` - 消息发送通道
    pub async fn subscribe_deals(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<()> {
        let ws_url = self.base_url.clone();
        println!("Connecting to MEXC WebSocket for deals: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ MEXC WebSocket connected successfully for deals");

        let (mut write, mut read) = ws_stream.split();

        // 发送订阅请求
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIPTION",
            "params": [
                format!("spot@public.aggre.deals.v3.api.pb@{}@{}", interval, symbol.to_uppercase())
            ]
        });

        let subscribe_text = subscribe_msg.to_string();
        println!("📤 发送成交订阅请求: {}", subscribe_text);
        
        let msg = WsMessage::Text(subscribe_text);
        write.send(msg).await?;

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                WsMessage::Text(text) => {
                    println!("📥 收到文本消息: {}", text);
                    
                    // 检查是否是订阅响应
                    if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(method) = response.get("method") {
                            if method == "SUBSCRIPTION" {
                                println!("✅ 成交订阅成功: {}", text);
                            }
                        }
                    }
                }
                WsMessage::Binary(data) => {
                    println!(" 收到成交二进制数据(protobuf)，长度: {}", data.len());
                    
                    // 尝试解析 MEXC 官方 protobuf 结构
                    match PushDataV3ApiWrapper::decode(&*data) {
                        Ok(wrapper) => {
                            if let Some(deals) = wrapper.extract_deals_data() {
                                println!("✅ 成交解析成功: {} | 成交笔数: {}", 
                                    wrapper.channel, deals.deals.len());
                                
                                // 处理每笔成交
                                for deal in &deals.deals {
                                    let trade_type_str = match deal.trade_type {
                                        1 => "买",
                                        2 => "卖",
                                        _ => "未知",
                                    };
                                    
                                    println!("   {} | 价格: {} | 数量: {} | 类型: {} | 时间: {}", 
                                        symbol.to_uppercase(), deal.price, deal.quantity, trade_type_str, deal.time);
                                    
                                    // 转换为 JSON 格式发送到通道
                                    let json_data = serde_json::json!({
                                        "symbol": wrapper.symbol.clone().unwrap_or_else(|| symbol.to_uppercase()),
                                        "price": deal.price.parse::<f64>().unwrap_or(0.0),
                                        "quantity": deal.quantity.parse::<f64>().unwrap_or(0.0),
                                        "trade_type": deal.trade_type,
                                        "trade_type_str": trade_type_str,
                                        "time": deal.time,
                                        "event_type": deals.event_type,
                                        "send_time": wrapper.send_time.unwrap_or(0),
                                        "timestamp": chrono::Utc::now().timestamp()
                                    });
                                    
                                    if let Err(e) = tx.send(json_data.to_string()) {
                                        eprintln!("Failed to send deal message: {}", e);
                                        break;
                                    }
                                }
                            } else {
                                println!("❌ 包装器中没有成交数据");
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ 成交数据 Protobuf 解析失败: {}", e);
                            // 如果解析失败，发送原始十六进制数据用于调试
                            let hex_data = hex::encode(&data);
                            if let Err(e) = tx.send(format!("DEALS_PARSE_ERROR:{}", hex_data)) {
                                eprintln!("Failed to send error message: {}", e);
                                break;
                            }
                        }
                    }
                }
                WsMessage::Close(_) => {
                    println!("❌ MEXC WebSocket connection closed");
                    break;
                }
                WsMessage::Ping(data) => {
                    println!(" 收到 Ping，发送 Pong 响应");
                    let pong_msg = WsMessage::Pong(data);
                    if let Err(e) = write.send(pong_msg).await {
                        eprintln!("Failed to send pong: {}", e);
                        break;
                    }
                }
                WsMessage::Pong(_) => {
                    println!("🏓 收到 Pong");
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 带重连机制的 K线订阅
    pub async fn subscribe_kline_with_reconnect(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<String>,
        max_retries: usize,
        retry_delay: Duration,
    ) -> Result<()> {
        let mut retry_count = 0;

        loop {
            match self.subscribe_kline(symbol, interval, tx.clone()).await {
                Ok(_) => {
                    println!("MEXC WebSocket connection completed normally");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    eprintln!(
                        "MEXC WebSocket connection failed (attempt {}/{}): {}",
                        retry_count, max_retries, e
                    );

                    if retry_count >= max_retries {
                        return Err(e);
                    }

                    println!("Retrying in {:?}...", retry_delay);
                    tokio::time::sleep(retry_delay).await;
                }
            }
        }

        Ok(())
    }

    /// 订阅多个交易对的 K线数据
    pub async fn subscribe_multiple_klines(
        &self,
        subscriptions: Vec<(String, String)>, // (symbol, interval)
        tx: mpsc::UnboundedSender<String>,
    ) -> Result<()> {
        let ws_url = self.base_url.clone();
        println!("Connecting to MEXC WebSocket for multiple subscriptions: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ MEXC WebSocket connected successfully for multiple subscriptions");

        let (mut write, mut read) = ws_stream.split();

        // 发送多个订阅请求
        for (symbol, interval) in &subscriptions {
            let subscribe_msg = serde_json::json!({
                "method": "SUBSCRIPTION",
                "params": [
                    format!("spot@public.kline.v3.api.pb@{}@{}", symbol.to_uppercase(), interval)
                ]
            });

            let subscribe_text = subscribe_msg.to_string();
            println!("📤 发送订阅请求: {} - {}", symbol, interval);
            
            let msg = WsMessage::Text(subscribe_text);
            write.send(msg).await?;
            
            // 短暂延迟，避免请求过快
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                WsMessage::Text(text) => {
                    println!("📥 收到文本消息: {}", text);
                    
                    // 发送到通道
                    if let Err(e) = tx.send(text.clone()) {
                        eprintln!("Failed to send message: {}", e);
                        break;
                    }
                }
                WsMessage::Binary(data) => {
                    println!(" 收到二进制数据(protobuf)，长度: {}", data.len());
                    
                    let hex_data = hex::encode(&data);
                    if let Err(e) = tx.send(format!("PROTOBUF_DATA:{}", hex_data)) {
                        eprintln!("Failed to send protobuf message: {}", e);
                        break;
                    }
                }
                WsMessage::Close(_) => {
                    println!("❌ MEXC WebSocket connection closed");
                    break;
                }
                WsMessage::Ping(data) => {
                    println!(" 收到 Ping，发送 Pong 响应");
                    let pong_msg = WsMessage::Pong(data);
                    if let Err(e) = write.send(pong_msg).await {
                        eprintln!("Failed to send pong: {}", e);
                        break;
                    }
                }
                WsMessage::Pong(_) => {
                    println!("🏓 收到 Pong");
                }
                _ => {}
            }
        }

        Ok(())
    }
}

// 使用示例和测试
#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_mexc_websocket_connection() {
        let ws = MexcWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 启动 WebSocket 连接
        let symbol = "BTCUSDT";
        let interval = "Min15";

        let ws_handle = tokio::spawn(async move { 
            ws.subscribe_kline(symbol, interval, tx).await 
        });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 5;

        while let Some(data) = rx.recv().await {
            println!("Received: {}", data);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }

    #[tokio::test]
    async fn test_multiple_klines_subscription() {
        let ws = MexcWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 订阅多个交易对
        let subscriptions = vec![
            ("BTCUSDT".to_string(), "Min15".to_string()),
            ("ETHUSDT".to_string(), "Min5".to_string()),
        ];

        let ws_handle = tokio::spawn(async move { 
            ws.subscribe_multiple_klines(subscriptions, tx).await 
        });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 10;

        while let Some(data) = rx.recv().await {
            println!("Received: {}", data);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }
}
