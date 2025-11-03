use crate::common::config::ws_config::{
    MarkPriceConfig, KlineConfig, PartialDepthConfig, DiffDepthConfig, ConfigLoader, BookTickerConfig
};
use super::ws::BinanceWebSocket;
use crate::dto::binance::websocket::{MarkPriceData, BinancePartialDepth, KlineData, BookTickerData};
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use crate::{websocket_log, system_log};

/// WebSocket 数据类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WebSocketDataType {
    MarkPrice,      // 标记价格
    Kline,          // K线数据
    PartialDepth,   // 部分订单簿深度
    DiffDepth,      // 订单簿深度差异
    BookTicker,     // Book Ticker数据
}

/// WebSocket 消息类型 - 使用 Arc 优化内存使用
#[derive(Debug, Clone)]
pub enum WebSocketMessage {
    MarkPrice(Arc<MarkPriceData>),
    Kline(Arc<KlineData>),
    PartialDepth(Arc<BinancePartialDepth>),
    DiffDepth(Arc<BinancePartialDepth>),
    BookTicker(Arc<BookTickerData>),
}

/// WebSocket 连接信息
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub connection_id: String,
    pub symbol: String,
    pub data_type: WebSocketDataType,
    pub status: ConnectionStatus,
    pub created_at: std::time::Instant,
    pub last_message_at: Option<std::time::Instant>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connecting,
    Connected,
    Disconnected,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct WebSocketManager {
    ws_client: BinanceWebSocket,
    connections: Arc<Mutex<HashMap<String, (JoinHandle<()>, ConnectionInfo)>>>,
    message_tx: mpsc::UnboundedSender<WebSocketMessage>,
}

impl WebSocketManager {
    pub fn new(message_tx: mpsc::UnboundedSender<WebSocketMessage>) -> Self {
        Self {
            ws_client: BinanceWebSocket::new(),
            connections: Arc::new(Mutex::new(HashMap::new())),
            message_tx,
        }
    }

    /// 启动标记价格 WebSocket 连接
    pub async fn start_mark_price(&self, config: MarkPriceConfig) -> Result<()> {
        let connection_id = format!("mark_price_{}_{}", config.symbol.join("_"), config.interval);
        let message_tx = self.message_tx.clone();
        
        let ws_client = self.ws_client.clone();
        let connections = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        
        // 克隆配置数据以避免生命周期问题
        let symbols = config.symbol.clone();
        let interval = config.interval.clone();
        let auto_reconnect = config.base.auto_reconnect;
        let max_retries = config.base.max_retries;
        let retry_delay = config.base.retry_delay();
        let tags = config.base.tags.clone();
        
        // 创建连接信息
        let connection_info = ConnectionInfo {
            connection_id: connection_id.clone(),
            symbol: symbols.join(","),  // 转换为字符串用于显示
            data_type: WebSocketDataType::MarkPrice,
            status: ConnectionStatus::Connecting,
            created_at: std::time::Instant::now(),
            last_message_at: None,
            tags: tags.clone(),
        };
        
        let handle = tokio::spawn(async move {
            let result: Result<(), Box<dyn std::error::Error + Send + Sync>> = if auto_reconnect {
                // 为每个交易对创建连接
                for symbol in &symbols {
                    // 创建专门用于 MarkPriceData 的通道
                    let (mark_price_tx, mut mark_price_rx) = mpsc::unbounded_channel::<MarkPriceData>();
                    
                    // 启动消息转发任务
                    let message_tx_clone = message_tx.clone();
                    tokio::spawn(async move {
                        while let Some(data) = mark_price_rx.recv().await {
                            if let Err(e) = message_tx_clone.send(WebSocketMessage::MarkPrice(Arc::new(data))) {
                                websocket_log!(warn, "Failed to forward mark price message: {}", e);
                                break;
                            }
                        }
                    });
                    
                    let result = ws_client
                        .subscribe_with_reconnect(
                            symbol,
                            &interval,
                            mark_price_tx,
                            max_retries,
                            retry_delay,
                        )
                        .await;
                    
                    if let Err(e) = result {
                        websocket_log!(warn, "Mark price connection failed: {} - {}", symbol, e);
                    }
                }
                Ok(())
            } else {
                // 为每个交易对创建连接
                for symbol in &symbols {
                    // 创建专门用于 MarkPriceData 的通道
                    let (mark_price_tx, mut mark_price_rx) = mpsc::unbounded_channel::<MarkPriceData>();
                    
                    // 启动消息转发任务
                    let message_tx_clone = message_tx.clone();
                    tokio::spawn(async move {
                        while let Some(data) = mark_price_rx.recv().await {
                            if let Err(e) = message_tx_clone.send(WebSocketMessage::MarkPrice(Arc::new(data))) {
                                websocket_log!(warn, "Failed to forward mark price message: {}", e);
                                break;
                            }
                        }
                    });
                    
                    let result = ws_client
                        .subscribe_mark_price(symbol, &interval, mark_price_tx)
                        .await;
                    
                    if let Err(e) = result {
                        websocket_log!(warn, "Mark price connection failed: {} - {}", symbol, e);
                    }
                }
                Ok(())
            };
            
            if let Err(e) = result {
                websocket_log!(error, "Mark price connection failed: {}", e);
                // 更新连接状态
                let mut conns = connections.lock().await;
                if let Some((_, info)) = conns.get_mut(&connection_id_clone) {
                    info.status = ConnectionStatus::Error(e.to_string());
                }
            }
            
            // 从连接映射中移除
            let mut conns = connections.lock().await;
            conns.remove(&connection_id_clone);
        });
        
        // 保存连接句柄和信息
        let mut conns = self.connections.lock().await;
        conns.insert(connection_id, (handle, connection_info));
        
        Ok(())
    }

    /// 启动 K线数据 WebSocket 连接（优化版本 - 批量订阅）
    pub async fn start_multi_kline(&self, config: KlineConfig) -> Result<()> {
        let connection_id = format!("multi_kline_{}_{}", config.symbol.join("_"), config.interval);
        let message_tx = self.message_tx.clone();
        
        let ws_client = self.ws_client.clone();
        let connections = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        
        // 克隆配置数据以避免生命周期问题
        let symbols = config.symbol.clone();
        let interval = config.interval.clone();
        let tags = config.base.tags.clone();
        
        // 创建连接信息
        let connection_info = ConnectionInfo {
            connection_id: connection_id.clone(),
            symbol: symbols.join(","),  // 转换为字符串用于显示
            data_type: WebSocketDataType::Kline,
            status: ConnectionStatus::Connecting,
            created_at: std::time::Instant::now(),
            last_message_at: None,
            tags: tags.clone(),
        };
        
        let handle = tokio::spawn(async move {
            // 创建专门用于 KlineData 的通道
            let (kline_tx, mut kline_rx) = mpsc::unbounded_channel::<KlineData>();
            
            // 启动消息转发任务
            let message_tx_clone = message_tx.clone();
            tokio::spawn(async move {
                while let Some(data) = kline_rx.recv().await {
                    if let Err(e) = message_tx_clone.send(WebSocketMessage::Kline(Arc::new(data))) {
                        websocket_log!(warn, "Failed to forward kline message: {}", e);
                        break;
                    }
                }
            });
            
            // 使用带重试的批量订阅方法
            let result = ws_client.subscribe_multiple_klines_with_reconnect(
                &symbols, 
                &interval, 
                kline_tx,
                3, // max_retries
                std::time::Duration::from_millis(100) // retry_delay
            ).await;
            
            if let Err(e) = result {
                websocket_log!(warn, "Multi kline connection failed: {}", e);
                // 更新连接状态
                let mut conns = connections.lock().await;
                if let Some((_, info)) = conns.get_mut(&connection_id_clone) {
                    info.status = ConnectionStatus::Error(e.to_string());
                }
            }
            
            // 从连接映射中移除
            let mut conns = connections.lock().await;
            conns.remove(&connection_id_clone);
        });
        
        // 保存连接句柄和信息
        let mut conns = self.connections.lock().await;
        conns.insert(connection_id, (handle, connection_info));
        
        Ok(())
    }

    /// 启动 K线数据 WebSocket 连接（向后兼容版本）
    pub async fn start_kline(&self, config: KlineConfig) -> Result<()> {
        let connection_id = format!("kline_{}_{}", config.symbol.join("_"), config.interval);
        let message_tx = self.message_tx.clone();
        
        let ws_client = self.ws_client.clone();
        let connections = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        
        // 克隆配置数据以避免生命周期问题
        let symbols = config.symbol.clone();
        let interval = config.interval.clone();
        let tags = config.base.tags.clone();
        
        // 创建连接信息
        let connection_info = ConnectionInfo {
            connection_id: connection_id.clone(),
            symbol: symbols.join(","),  // 转换为字符串用于显示
            data_type: WebSocketDataType::Kline,
            status: ConnectionStatus::Connecting,
            created_at: std::time::Instant::now(),
            last_message_at: None,
            tags: tags.clone(),
        };
        
        let handle = tokio::spawn(async move {
            // 为每个交易对创建连接
            for symbol in &symbols {
                // 创建专门用于 KlineData 的通道
                let (kline_tx, mut kline_rx) = mpsc::unbounded_channel::<KlineData>();
                
                // 启动消息转发任务
                let message_tx_clone = message_tx.clone();
                tokio::spawn(async move {
                    while let Some(data) = kline_rx.recv().await {
                        if let Err(e) = message_tx_clone.send(WebSocketMessage::Kline(Arc::new(data))) {
                            websocket_log!(warn, "Failed to forward kline message: {}", e);
                            break;
                        }
                    }
                });
                
                let result = ws_client.subscribe_kline_with_reconnect(
                    symbol, 
                    &interval, 
                    kline_tx,
                    3, // max_retries
                    std::time::Duration::from_millis(100) // retry_delay
                ).await;
                
                if let Err(e) = result {
                    websocket_log!(warn, "Kline connection failed: {} - {}", symbol, e);
                }
            }
            
            // 从连接映射中移除
            let mut conns = connections.lock().await;
            conns.remove(&connection_id_clone);
        });
        
        // 保存连接句柄和信息
        let mut conns = self.connections.lock().await;
        conns.insert(connection_id, (handle, connection_info));
        
        Ok(())
    }

    /// 启动部分订单簿深度 WebSocket 连接
    pub async fn start_partial_depth(&self, config: PartialDepthConfig) -> Result<()> {
        let connection_id = format!("partial_depth_{}_{}_{}", config.symbol.join("_"), config.levels, config.interval);
        let message_tx = self.message_tx.clone();
        
        let ws_client = self.ws_client.clone();
        let connections = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        
        // 克隆配置数据以避免生命周期问题
        let symbols = config.symbol.clone();
        let _levels = config.levels;
        let interval = config.interval.clone();
        let tags = config.base.tags.clone();
        
        // 创建连接信息
        let connection_info = ConnectionInfo {
            connection_id: connection_id.clone(),
            symbol: symbols.join(","),  // 转换为字符串用于显示
            data_type: WebSocketDataType::PartialDepth,
            status: ConnectionStatus::Connecting,
            created_at: std::time::Instant::now(),
            last_message_at: None,
            tags: tags.clone(),
        };
        
        let handle = tokio::spawn(async move {
            // 为每个交易对创建连接
            for symbol in &symbols {
                // 创建专门用于 BinanceDepth 的通道
                let (depth_tx, mut depth_rx) = mpsc::unbounded_channel::<BinancePartialDepth>();
                
                // 启动消息转发任务
                let message_tx_clone = message_tx.clone();
                tokio::spawn(async move {
                    while let Some(data) = depth_rx.recv().await {
                        if let Err(e) = message_tx_clone.send(WebSocketMessage::PartialDepth(Arc::new(data))) {
                            websocket_log!(warn, "Failed to forward partial depth message: {}", e);
                            break;
                        }
                    }
                });
                
                let result = ws_client.subscribe_depth(symbol, &interval, depth_tx).await;
                
                if let Err(e) = result {
                    websocket_log!(warn, "Partial depth connection failed: {} - {}", symbol, e);
                }
            }
            
            // 从连接映射中移除
            let mut conns = connections.lock().await;
            conns.remove(&connection_id_clone);
        });
        
        // 保存连接句柄和信息
        let mut conns = self.connections.lock().await;
        conns.insert(connection_id, (handle, connection_info));
        
        Ok(())
    }

    /// 启动订单簿深度差异 WebSocket 连接
    pub async fn start_diff_depth(&self, config: DiffDepthConfig) -> Result<()> {
        let connection_id = format!("diff_depth_{}_{}", config.symbol.join("_"), config.level);
        let message_tx = self.message_tx.clone();
        
        let ws_client = self.ws_client.clone();
        let connections = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        
        // 克隆配置数据以避免生命周期问题
        let symbols = config.symbol.clone();
        let _level = config.level;
        let tags = config.base.tags.clone();
        
        // 创建连接信息
        let connection_info = ConnectionInfo {
            connection_id: connection_id.clone(),
            symbol: symbols.join(","),  // 转换为字符串用于显示
            data_type: WebSocketDataType::DiffDepth,
            status: ConnectionStatus::Connecting,
            created_at: std::time::Instant::now(),
            last_message_at: None,
            tags: tags.clone(),
        };
        
        let handle = tokio::spawn(async move {
            // 为每个交易对创建连接
            for symbol in &symbols {
                // 创建专门用于 BinanceDepth 的通道
                let (depth_tx, mut depth_rx) = mpsc::unbounded_channel::<BinancePartialDepth>();
                
                // 启动消息转发任务
                let message_tx_clone = message_tx.clone();
                tokio::spawn(async move {
                    while let Some(data) = depth_rx.recv().await {
                        if let Err(e) = message_tx_clone.send(WebSocketMessage::DiffDepth(Arc::new(data))) {
                            websocket_log!(warn, "Failed to forward diff depth message: {}", e);
                            break;
                        }
                    }
                });
                
                // 这里需要根据实际的WebSocket客户端API来调用相应的方法
                // 暂时使用subscribe_depth作为占位符
                let result = ws_client.subscribe_depth(symbol, "100ms", depth_tx).await;
                
                if let Err(e) = result {
                    websocket_log!(warn, "Diff depth connection failed: {} - {}", symbol, e);
                }
            }
            
            // 从连接映射中移除
            let mut conns = connections.lock().await;
            conns.remove(&connection_id_clone);
        });
        
        // 保存连接句柄和信息
        let mut conns = self.connections.lock().await;
        conns.insert(connection_id, (handle, connection_info));
        
        Ok(())
    }

    /// 启动 Book Ticker WebSocket 连接
    pub async fn start_book_ticker(&self, config: BookTickerConfig) -> Result<()> {
        let connection_id = format!("book_ticker_{}", config.symbol.join("_"));
        let message_tx = self.message_tx.clone();
        
        let ws_client = self.ws_client.clone();
        let connections = self.connections.clone();
        let connection_id_clone = connection_id.clone();
        
        // 克隆配置数据以避免生命周期问题
        let symbols = config.symbol.clone();
        let tags = config.base.tags.clone();
        
        // 创建连接信息
        let connection_info = ConnectionInfo {
            connection_id: connection_id.clone(),
            symbol: symbols.join(","),  // 转换为字符串用于显示
            data_type: WebSocketDataType::BookTicker,
            status: ConnectionStatus::Connecting,
            created_at: std::time::Instant::now(),
            last_message_at: None,
            tags: tags.clone(),
        };
        
        let handle = tokio::spawn(async move {
            // 为每个交易对创建连接
            for symbol in &symbols {
                // 创建专门用于 BookTickerData 的通道
                let (book_ticker_tx, mut book_ticker_rx) = mpsc::unbounded_channel::<BookTickerData>();
                
                // 启动消息转发任务
                let message_tx_clone = message_tx.clone();
                tokio::spawn(async move {
                    while let Some(data) = book_ticker_rx.recv().await {
                        if let Err(e) = message_tx_clone.send(WebSocketMessage::BookTicker(Arc::new(data))) {
                            websocket_log!(warn, "Failed to forward book ticker message: {}", e);
                            break;
                        }
                    }
                });
                
                let result = ws_client.subscribe_book_ticker(symbol, book_ticker_tx).await;
                
                if let Err(e) = result {
                    websocket_log!(warn, "Book ticker connection failed: {} - {}", symbol, e);
                }
            }
            
            // 从连接映射中移除
            let mut conns = connections.lock().await;
            conns.remove(&connection_id_clone);
        });
        
        // 保存连接句柄和信息
        let mut conns = self.connections.lock().await;
        conns.insert(connection_id, (handle, connection_info));
        
        Ok(())
    }

    /// 从配置文件启动所有连接
    pub async fn start_from_config(&self, config_path: &str) -> Result<()> {
        let configs = ConfigLoader::load_from_file(config_path)
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;
        
        // 启动标记价格连接
        for config in configs.mark_price {
            self.start_mark_price(config).await?;
        }
        
        // 启动K线连接
        for config in configs.kline {
            self.start_kline(config).await?;
        }
        
        // 启动部分深度连接
        for config in configs.partial_depth {
            self.start_partial_depth(config).await?;
        }
        
        // 启动深度差异连接
        for config in configs.diff_depth {
            self.start_diff_depth(config).await?;
        }
        
        // 启动 Book Ticker 连接
        for config in configs.book_ticker {
            self.start_book_ticker(config).await?;
        }
        
        Ok(())
    }

    /// 停止指定的连接
    pub async fn stop_connection(&self, connection_id: &str) -> Result<()> {
        let mut conns = self.connections.lock().await;
        
        if let Some((handle, _)) = conns.remove(connection_id) {
            handle.abort();
            system_log!(info, "Stopped connection: {}", connection_id);
        }
        
        Ok(())
    }

    /// 停止所有连接
    pub async fn stop_all_connections(&self) -> Result<()> {
        let mut conns = self.connections.lock().await;
        
        for (connection_id, (handle, _)) in conns.drain() {
            handle.abort();
            system_log!(info, "Stopped connection: {}", connection_id);
        }
        
        Ok(())
    }

    /// 获取活跃连接数量
    pub async fn active_connections(&self) -> usize {
        let conns = self.connections.lock().await;
        conns.len()
    }

    /// 获取活跃连接列表
    pub async fn list_connections(&self) -> Vec<ConnectionInfo> {
        let conns = self.connections.lock().await;
        conns.values().map(|(_, info)| info.clone()).collect()
    }

    /// 根据标签获取连接
    pub async fn get_connections_by_tag(&self, tag: &str) -> Vec<ConnectionInfo> {
        let conns = self.connections.lock().await;
        conns.values()
            .filter_map(|(_, info)| {
                if info.tags.contains(&tag.to_string()) {
                    Some(info.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// 根据数据类型获取连接
    pub async fn get_connections_by_type(&self, data_type: &WebSocketDataType) -> Vec<ConnectionInfo> {
        let conns = self.connections.lock().await;
        conns.values()
            .filter_map(|(_, info)| {
                if &info.data_type == data_type {
                    Some(info.clone())
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// 根据交易对获取连接
    pub async fn get_connections_by_symbol(&self, symbol: &str) -> Vec<ConnectionInfo> {
        let conns = self.connections.lock().await;
        conns.values()
            .filter_map(|(_, info)| {
                if info.symbol == symbol {
                    Some(info.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

// 便捷的工厂函数
pub async fn create_websocket_manager() -> Result<(WebSocketManager, mpsc::UnboundedReceiver<WebSocketMessage>)> {
    let (tx, rx) = mpsc::unbounded_channel();
    let manager = WebSocketManager::new(tx);
    Ok((manager, rx))
}

// 使用示例
pub async fn example_usage() -> Result<()> {
    let (manager, mut rx) = create_websocket_manager().await?;
    
    // 从配置文件启动连接
    manager.start_from_config("config.toml").await?;
    
    // 处理消息
    tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            match data {
                WebSocketMessage::MarkPrice(mark_price) => {
                    websocket_log!(info, "Received mark price: {:?}", mark_price);
                },
                WebSocketMessage::Kline(kline) => {
                    websocket_log!(info, "Received kline: {:?}", kline);
                },
                WebSocketMessage::PartialDepth(depth) => {
                    websocket_log!(info, "Received partial depth: {:?}", depth);
                },
                WebSocketMessage::DiffDepth(depth) => {
                    websocket_log!(info, "Received diff depth: {:?}", depth);
                },
                WebSocketMessage::BookTicker(book_ticker) => {
                    websocket_log!(info, "Received book ticker: {:?}", book_ticker);
                },
            }
        }
    });
    
    // 保持运行一段时间
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    // 停止所有连接
    manager.stop_all_connections().await?;
    
    Ok(())
} 