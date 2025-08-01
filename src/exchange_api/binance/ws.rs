use crate::common::consts::BINANCE_WS;
use crate::dto::binance::websocket::{MarkPriceData, DepthUpdateData, KlineData};
use anyhow::Result;
use futures::StreamExt;
use serde_json;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

#[derive(Debug, Clone)]
pub struct BinanceWebSocket {
    base_url: String,
}

impl BinanceWebSocket {
    pub fn new() -> Self {
        Self {
            base_url: BINANCE_WS.to_string(),
        }
    }

    /// 创建标记价格 WebSocket 连接
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "bnbusdt"
    /// * `interval` - 更新间隔，如 "1s", "1m", "5m"
    /// * `tx` - 消息发送通道
    pub async fn subscribe_mark_price(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<MarkPriceData>,
    ) -> Result<()> {
        let stream_name = format!("{}@markPrice@{}", symbol, interval);
        let ws_url = format!("{}/{}", self.base_url, stream_name);

        println!("Connecting to WebSocket: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ WebSocket connected successfully");

        let (_, mut read) = ws_stream.split();

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<MarkPriceData>(&text) {
                        if let Err(e) = tx.send(data) {
                            eprintln!("Failed to send message: {}", e);
                            break;
                        }
                    }
                }
                Message::Close(_) => {
                    println!("WebSocket connection closed");
                    break;
                }
                Message::Ping(_data) => {
                    // 可以在这里发送 pong 响应
                    println!("Received ping");
                }
                Message::Pong(_) => {
                    println!("Received pong");
                }
                _ => {
                    // 忽略其他类型的消息
                }
            }
        }

        Ok(())
    }

    /// 订阅订单簿深度数据
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "btcusdt"
    /// * `interval` - 更新间隔，如 "250ms", "500ms", "100ms"
    /// * `tx` - 消息发送通道
    pub async fn subscribe_depth(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<DepthUpdateData>,
    ) -> Result<()> {
        let stream_name = if interval == "250ms" {
            format!("{}@depth", symbol)
        } else {
            format!("{}@depth@{}", symbol, interval)
        };

        let ws_url = format!("{}/{}", self.base_url, stream_name);

        println!("Connecting to Depth WebSocket: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ Depth WebSocket connected successfully");

        let (_, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<DepthUpdateData>(&text) {
                        if let Err(e) = tx.send(data) {
                            eprintln!("Failed to send depth message: {}", e);
                            break;
                        }
                    }
                }
                Message::Close(_) => {
                    println!("Depth WebSocket connection closed");
                    break;
                }
                Message::Ping(_) => {
                    println!("Received ping from depth stream");
                }
                Message::Pong(_) => {
                    println!("Received pong from depth stream");
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 创建多个标记价格 WebSocket 连接
    pub async fn subscribe_multiple_mark_prices(
        &self,
        symbols: Vec<String>,
        interval: &str,
        tx: mpsc::UnboundedSender<MarkPriceData>,
    ) -> Result<()> {
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|symbol| format!("{}@markPrice@{}", symbol, interval))
            .collect();

        let combined_stream = stream_names.join("/");
        let ws_url = format!("{}/{}", self.base_url, combined_stream);

        println!("Connecting to multiple streams: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ Multiple WebSocket streams connected successfully");

        let (_, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<MarkPriceData>(&text) {
                        if let Err(e) = tx.send(data) {
                            eprintln!("Failed to send message: {}", e);
                            break;
                        }
                    }
                }
                Message::Close(_) => {
                    println!("WebSocket connection closed");
                    break;
                }
                Message::Ping(_) => {
                    println!("Received ping");
                }
                Message::Pong(_) => {
                    println!("Received pong");
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 订阅多个交易对的深度数据
    pub async fn subscribe_multiple_depths(
        &self,
        symbols: &[String],
        interval: &str,
        tx: mpsc::UnboundedSender<DepthUpdateData>,
    ) -> Result<()> {
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|symbol| {
                if interval == "250ms" {
                    format!("{}@depth", symbol)
                } else {
                    format!("{}@depth@{}", symbol, interval)
                }
            })
            .collect();

        let combined_stream = stream_names.join("/");
        let ws_url = format!("{}/{}", self.base_url, combined_stream);

        println!("Connecting to multiple depth streams: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ Multiple depth streams connected successfully");

        let (_, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<DepthUpdateData>(&text) {
                        if let Err(e) = tx.send(data) {
                            eprintln!("Failed to send depth message: {}", e);
                            break;
                        }
                    }
                }
                Message::Close(_) => {
                    println!("Multiple depth streams connection closed");
                    break;
                }
                Message::Ping(_) => {
                    println!("Received ping from multiple depth streams");
                }
                Message::Pong(_) => {
                    println!("Received pong from multiple depth streams");
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 带重连机制的 WebSocket 连接
    pub async fn subscribe_with_reconnect(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<MarkPriceData>,
        max_retries: usize,
        retry_delay: Duration,
    ) -> Result<()> {
        let mut retry_count = 0;

        loop {
            match self
                .subscribe_mark_price(symbol, interval, tx.clone())
                .await
            {
                Ok(_) => {
                    println!("WebSocket connection completed normally");
                    break;
                }
                Err(e) => {
                    retry_count += 1;
                    eprintln!(
                        "WebSocket connection failed (attempt {}/{}): {}",
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

    /// 订阅 Kline/Candlestick 数据
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "btcusdt"
    /// * `interval` - K线间隔，如 "1m", "5m", "1h", "1d"
    /// * `tx` - 消息发送通道
    pub async fn subscribe_kline(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<KlineData>,
    ) -> Result<()> {
        let stream_name = format!("{}@kline_{}", symbol, interval);
        let ws_url = format!("{}/{}", self.base_url, stream_name);

        println!("Connecting to Kline WebSocket: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ Kline WebSocket connected successfully");

        let (_, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<KlineData>(&text) {
                        if let Err(e) = tx.send(data) {
                            eprintln!("Failed to send kline message: {}", e);
                            break;
                        }
                    }
                }
                Message::Close(_) => {
                    println!("Kline WebSocket connection closed");
                    break;
                }
                Message::Ping(_) => {
                    println!("Received ping from kline stream");
                }
                Message::Pong(_) => {
                    println!("Received pong from kline stream");
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// 订阅多个交易对的 Kline 数据
    pub async fn subscribe_multiple_klines(
        &self,
        symbols: &[String],
        interval: &str,
        tx: mpsc::UnboundedSender<KlineData>,
    ) -> Result<()> {
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|symbol| format!("{}@kline_{}", symbol, interval))
            .collect();

        let combined_stream = stream_names.join("/");
        let ws_url = format!("{}/{}", self.base_url, combined_stream);

        println!("Connecting to multiple kline streams: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ Multiple kline streams connected successfully");

        let (_, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<KlineData>(&text) {
                        if let Err(e) = tx.send(data) {
                            eprintln!("Failed to send kline message: {}", e);
                            break;
                        }
                    }
                }
                Message::Close(_) => {
                    println!("Multiple kline streams connection closed");
                    break;
                }
                Message::Ping(_) => {
                    println!("Received ping from multiple kline streams");
                }
                Message::Pong(_) => {
                    println!("Received pong from multiple kline streams");
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
    use crate::dto::binance::websocket::DepthUpdateData;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_websocket_connection() {
        let ws = BinanceWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 启动 WebSocket 连接
        let symbol = "bnbusdt";
        let interval = "1s";

        let ws_handle =
            tokio::spawn(async move { ws.subscribe_mark_price(symbol, interval, tx).await });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 5;

        while let Some(data) = rx.recv().await {
            println!("Received: {:?}", data);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }

    #[test]
    fn test_serialization_performance() {
        let data = MarkPriceData {
            symbol: "BTCUSDT".to_string(),
            mark_price: 50000.0,
            index_price: 50001.0,
            estimated_settle_price: 50000.5,
            last_funding_rate: 0.0001,
            next_funding_time: 1640995200000,
            interest_rate: 0.0001,
            time: 1640995200000,
        };

        let iterations = 100_000;
        let start = Instant::now();

        for _ in 0..iterations {
            let _json = serde_json::to_string(&data).unwrap();
        }

        let elapsed = start.elapsed();
        println!("序列化 {} 次耗时: {:?}", iterations, elapsed);
        println!("平均每次序列化: {:?}", elapsed / iterations);

        // 典型结果：每次序列化约 1-5 微秒
    }

    #[test]
    fn test_depth_data_parsing() {
        let json_str = r#"{
            "e": "depthUpdate",
            "E": 1750216875946,
            "T": 1750216875937,
            "s": "ETHUSDT",
            "U": 7818596781509,
            "u": 7818596794961,
            "pu": 7818596780926,
            "b": [["200.00", "260.401"]],
            "a": [["2521.13", "37.315"]]
        }"#;

        let data: DepthUpdateData = serde_json::from_str(json_str).unwrap();

        assert_eq!(data.symbol, "ETHUSDT");
        assert_eq!(data.event_type, "depthUpdate");
        assert_eq!(data.bids.len(), 1);
        assert_eq!(data.asks.len(), 1);
        
        // 测试自动转换后的数值类型
        assert_eq!(data.bids[0][0], 200.0);
        assert_eq!(data.bids[0][1], 260.401);
        assert_eq!(data.asks[0][0], 2521.13);
        assert_eq!(data.asks[0][1], 37.315);
    }

    #[tokio::test]
    async fn test_depth_websocket_connection() {
        let ws = BinanceWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 启动深度数据 WebSocket 连接
        let symbol = "btcusdt";
        let interval = "250ms";

        let ws_handle = tokio::spawn(async move { ws.subscribe_depth(symbol, interval, tx).await });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 3;

        while let Some(data) = rx.recv().await {
            println!("收到深度数据: {}", data.symbol);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }

    #[tokio::test]
    async fn test_kline_websocket_connection() {
        let ws = BinanceWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 启动 Kline WebSocket 连接
        let symbol = "btcusdt";
        let interval = "1m";

        let ws_handle = tokio::spawn(async move { ws.subscribe_kline(symbol, interval, tx).await });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 3;

        while let Some(data) = rx.recv().await {
            println!("收到K线数据: {} - {}", data.symbol, data.kline.interval);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }

    #[tokio::test]
    async fn test_multiple_klines_websocket_connection() {
        let ws = BinanceWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // 启动多个 Kline WebSocket 连接
        let symbols = vec!["btcusdt".to_string(), "ethusdt".to_string()];
        let interval = "5m";

        let ws_handle =
            tokio::spawn(async move { ws.subscribe_multiple_klines(&symbols, interval, tx).await });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 5;

        while let Some(data) = rx.recv().await {
            println!("收到多K线数据: {} - {}", data.symbol, data.kline.interval);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }
}
