use crate::common::consts::BINANCE_WS;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;

use super::ws::{BinanceWebSocket, MarkPriceData};

#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub symbol: String,
    pub interval: String,
    pub auto_reconnect: bool,
    pub max_retries: usize,
    pub retry_delay: Duration,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            symbol: "bnbusdt".to_string(),
            interval: "1s".to_string(),
            auto_reconnect: true,
            max_retries: 5,
            retry_delay: Duration::from_secs(5),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WebSocketManager {
    ws_client: BinanceWebSocket,
    connections: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    message_tx: mpsc::UnboundedSender<MarkPriceData>,
}

impl WebSocketManager {
    pub fn new(message_tx: mpsc::UnboundedSender<MarkPriceData>) -> Self {
        Self {
            ws_client: BinanceWebSocket::new(),
            connections: Arc::new(Mutex::new(HashMap::new())),
            message_tx,
        }
    }

    /// 启动单个 WebSocket 连接
    pub async fn start_connection(&self, config: WebSocketConfig) -> Result<()> {
        let connection_id = format!("{}_{}", config.symbol, config.interval);
        let message_tx = self.message_tx.clone();
        
        let ws_client = self.ws_client.clone();
        let connections = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        
        let handle = tokio::spawn(async move {
            let result = if config.auto_reconnect {
                ws_client
                    .subscribe_with_reconnect(
                        &config.symbol,
                        &config.interval,
                        message_tx,
                        config.max_retries,
                        config.retry_delay,
                    )
                    .await
            } else {
                ws_client
                    .subscribe_mark_price(&config.symbol, &config.interval, message_tx)
                    .await
            };
            
            if let Err(e) = result {
                eprintln!("WebSocket connection failed: {}", e);
            }
            
            // 从连接映射中移除
            let mut conns = connections.lock().await;
            conns.remove(&connection_id_clone);
        });
        
        // 保存连接句柄
        let mut conns = self.connections.lock().await;
        conns.insert(connection_id, handle);
        
        Ok(())
    }

    /// 启动多个 WebSocket 连接
    pub async fn start_multiple_connections(&self, configs: Vec<WebSocketConfig>) -> Result<()> {
        for config in configs {
            self.start_connection(config).await?;
        }
        Ok(())
    }

    /// 停止指定的连接
    pub async fn stop_connection(&self, symbol: &str, interval: &str) -> Result<()> {
        let connection_id = format!("{}_{}", symbol, interval);
        let mut conns = self.connections.lock().await;
        
        if let Some(handle) = conns.remove(&connection_id) {
            handle.abort();
            println!("Stopped connection: {}", connection_id);
        }
        
        Ok(())
    }

    /// 停止所有连接
    pub async fn stop_all_connections(&self) -> Result<()> {
        let mut conns = self.connections.lock().await;
        
        for (connection_id, handle) in conns.drain() {
            handle.abort();
            println!("Stopped connection: {}", connection_id);
        }
        
        Ok(())
    }

    /// 获取活跃连接数量
    pub async fn active_connections(&self) -> usize {
        let conns = self.connections.lock().await;
        conns.len()
    }

    /// 获取活跃连接列表
    pub async fn list_connections(&self) -> Vec<String> {
        let conns = self.connections.lock().await;
        conns.keys().cloned().collect()
    }
}

// 便捷的工厂函数
pub async fn create_websocket_manager() -> Result<(WebSocketManager, mpsc::UnboundedReceiver<MarkPriceData>)> {
    let (tx, rx) = mpsc::unbounded_channel();
    let manager = WebSocketManager::new(tx);
    Ok((manager, rx))
}

// 使用示例
pub async fn example_usage() -> Result<()> {
    let (manager, mut rx) = create_websocket_manager().await?;
    
    // 配置连接
    let configs = vec![
        WebSocketConfig {
            symbol: "bnbusdt".to_string(),
            interval: "1s".to_string(),
            auto_reconnect: true,
            max_retries: 5,
            retry_delay: Duration::from_secs(5),
        },
        WebSocketConfig {
            symbol: "btcusdt".to_string(),
            interval: "1s".to_string(),
            auto_reconnect: true,
            max_retries: 5,
            retry_delay: Duration::from_secs(5),
        },
    ];
    
    // 启动连接
    manager.start_multiple_connections(configs).await?;
    
    // 处理消息
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            println!("Received mark price: {:?}", data);
            // 在这里处理接收到的数据
        }
    });
    
    // 保持运行一段时间
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    // 停止所有连接
    manager.stop_all_connections().await?;
    
    Ok(())
} 