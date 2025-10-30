use crate::common::consts::ASTER_WS;
use crate::dto::aster::websocket::AsterBookTickerData;
use crate::websocket_log;
use anyhow::Result;
use futures::{StreamExt, SinkExt};
use serde::Deserialize;
use serde_json;
use tokio::sync::mpsc;
use tokio::time::{Duration, interval};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use url::Url;

/// ASTER WebSocket 客户端
#[derive(Debug, Clone)]
pub struct AsterWebSocket {
    base_url: String,
}

impl AsterWebSocket {
    /// 创建新的 ASTER WebSocket 客户端
    pub fn new() -> Self {
        Self {
            base_url: ASTER_WS.to_string(),
        }
    }

    /// 从指定 URL 创建 WebSocket 客户端
    pub fn build_from_url(url: &str) -> Self {
        Self {
            base_url: url.to_string(),
        }
    }

    /// 订阅 Book Ticker 数据
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "btcusdt"（需要小写）
    /// * `tx` - 消息发送通道
    ///
    /// # Notes
    /// - 服务端每5分钟会发送ping帧，客户端应当在15分钟内回复pong帧
    /// - 单个连接最多可以订阅 200 个Streams
    /// - WebSocket服务器每秒最多接受10个订阅消息
    pub async fn subscribe_book_ticker(
        &self,
        symbol: &str,
        tx: mpsc::UnboundedSender<AsterBookTickerData>,
    ) -> Result<()> {
        // stream名称需要小写
        let stream_name = format!("{}@bookTicker", symbol.to_lowercase());
        // 单个stream格式: /ws/<streamName>
        let ws_url = format!("{}/ws/{}", self.base_url, stream_name);

        websocket_log!(info, "Connecting to ASTER Book Ticker WebSocket: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        websocket_log!(info, "ASTER Book Ticker WebSocket connected successfully");

        let (write, mut read) = ws_stream.split();

        // 创建 pong 发送任务
        // 服务端每5分钟发送ping，客户端需要在15分钟内回复pong
        // 我们每10分钟发送一次pong作为心跳
        let mut write_clone = write;
        let pong_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(600)); // 10分钟
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                interval.tick().await;
                // 发送pong保持连接
                if let Err(e) = write_clone.send(Message::Pong(vec![])).await {
                    websocket_log!(warn, "Failed to send pong: {}", e);
                    break;
                }
                websocket_log!(debug, "Sent pong to ASTER WebSocket");
            }
        });

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    if let Ok(data) = serde_json::from_str::<AsterBookTickerData>(&text) {
                        if let Err(e) = tx.send(data) {
                            websocket_log!(warn, "Failed to send book ticker message: {}", e);
                            break;
                        }
                    } else {
                        websocket_log!(warn, "Failed to parse book ticker message: {}", text);
                    }
                }
                Message::Close(_) => {
                    websocket_log!(info, "ASTER Book Ticker WebSocket connection closed");
                    break;
                }
                Message::Ping(_) => {
                    websocket_log!(debug, "Received ping from ASTER book ticker stream");
                    // 立即回复pong
                    // 注意：read和write已经split，这里无法直接回复
                    // 但pong_handle任务会定期发送pong，应该足够
                }
                Message::Pong(_) => {
                    websocket_log!(debug, "Received pong from ASTER book ticker stream");
                }
                _ => {}
            }
        }

        // 停止pong任务
        pong_handle.abort();

        Ok(())
    }

    /// 订阅多个交易对的 Book Ticker 数据
    ///
    /// # Arguments
    /// * `symbols` - 交易对符号列表
    /// * `tx` - 消息发送通道
    ///
    /// # Notes
    /// - 组合streams的URL格式: /stream?streams=<streamName1>/<streamName2>/<streamName3>
    /// - 事件payload会以这样的格式封装: {"stream":"<streamName>","data":<rawPayload>}
    pub async fn subscribe_multiple_book_tickers(
        &self,
        symbols: &[String],
        tx: mpsc::UnboundedSender<AsterBookTickerData>,
    ) -> Result<()> {
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|symbol| format!("{}@bookTicker", symbol.to_lowercase()))
            .collect();

        let combined_stream = stream_names.join("/");
        let ws_url = format!("{}/stream?streams={}", self.base_url, combined_stream);

        websocket_log!(info, "Connecting to multiple ASTER book ticker streams: {}", ws_url);

        let url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        websocket_log!(info, "Multiple ASTER book ticker streams connected successfully");

        let (write, mut read) = ws_stream.split();

        // 创建 pong 发送任务
        let mut write_clone = write;
        let pong_handle = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(600)); // 10分钟
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            
            loop {
                interval.tick().await;
                if let Err(e) = write_clone.send(Message::Pong(vec![])).await {
                    websocket_log!(warn, "Failed to send pong: {}", e);
                    break;
                }
                websocket_log!(debug, "Sent pong to ASTER WebSocket");
            }
        });

        // 处理组合stream的格式: {"stream":"<streamName>","data":<rawPayload>}
        #[derive(Deserialize)]
        struct StreamWrapper {
            #[serde(rename = "stream")]
            _stream: String, // stream名称，解析时使用但不实际使用
            data: serde_json::Value,
        }

        while let Some(msg) = read.next().await {
            match msg? {
                Message::Text(text) => {
                    // 尝试解析为组合stream格式
                    if let Ok(wrapper) = serde_json::from_str::<StreamWrapper>(&text) {
                        // 提取实际的payload数据
                        if let Ok(data) = serde_json::from_value::<AsterBookTickerData>(wrapper.data) {
                            if let Err(e) = tx.send(data) {
                                websocket_log!(warn, "Failed to send book ticker message: {}", e);
                                break;
                            }
                        } else {
                            websocket_log!(warn, "Failed to parse book ticker data from wrapper: {}", text);
                        }
                    } else {
                        // 如果不是组合stream格式，尝试直接解析
                        if let Ok(data) = serde_json::from_str::<AsterBookTickerData>(&text) {
                            if let Err(e) = tx.send(data) {
                                websocket_log!(warn, "Failed to send book ticker message: {}", e);
                                break;
                            }
                        } else {
                            websocket_log!(warn, "Failed to parse book ticker message: {}", text);
                        }
                    }
                }
                Message::Close(_) => {
                    websocket_log!(info, "Multiple ASTER book ticker streams connection closed");
                    break;
                }
                Message::Ping(_) => {
                    websocket_log!(debug, "Received ping from multiple ASTER book ticker streams");
                }
                Message::Pong(_) => {
                    websocket_log!(debug, "Received pong from multiple ASTER book ticker streams");
                }
                _ => {}
            }
        }

        // 停止pong任务
        pong_handle.abort();

        Ok(())
    }
}

