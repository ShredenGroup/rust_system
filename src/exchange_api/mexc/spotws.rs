use futures::{StreamExt, SinkExt}; // 添加 SinkExt
use serde_json;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as WsMessage}; // 重命名
use url::Url;

// 导入 MEXC 的 protobuf 结构体和 Message trait
use crate::dto::mexc::PushDataV3ApiWrapper;
use prost::Message as ProstMessage; // 重命名
use crate::common::ts::BookTickerData;

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
    ) -> anyhow::Result<()> {
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
                            println!("✅ 解析成功: {} | 消息类型: {}", 
                                wrapper.channel, wrapper.get_message_type());
                            
                            if let Some(kline) = wrapper.extract_kline_data() {
                                println!("📊 K线数据: 开盘: {} | 收盘: {} | 最高: {} | 最低: {} | 成交量: {}", 
                                    kline.opening_price, kline.closing_price, 
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
                                println!("ℹ️ 未找到 K线数据，消息类型: {}", wrapper.get_message_type());
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
                    eprintln!("❌ MEXC WebSocket connection closed");
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
    ) -> anyhow::Result<()> {
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
                            println!("✅ 成交解析成功: {} | 消息类型: {}", 
                                wrapper.channel, wrapper.get_message_type());
                            
                            if let Some(deals) = wrapper.extract_deals_data() {
                                println!("📈 成交数据: 成交笔数: {}", deals.deals.len());
                                
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
                                println!("ℹ️ 未找到成交数据，消息类型: {}", wrapper.get_message_type());
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
                    eprintln!("❌ MEXC WebSocket connection closed");
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

    /// 订阅单个 Book Ticker 数据
    pub async fn subscribe_book_ticker(
        &mut self,
        symbol: &str,
        interval: &str,
    ) -> anyhow::Result<mpsc::Receiver<PushDataV3ApiWrapper>> {
        let (tx, rx) = mpsc::channel::<PushDataV3ApiWrapper>(1000);
        let ws_url = self.base_url.clone();
        println!("Connecting to MEXC Book Ticker WebSocket: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ MEXC Book Ticker WebSocket connected successfully");

        let (mut write, mut read) = ws_stream.split();

        // 发送订阅请求
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIPTION",
            "params": [
                format!("spot@public.aggre.bookTicker.v3.api.pb@{}@{}", interval, symbol.to_uppercase())
            ]
        });

        let subscribe_text = subscribe_msg.to_string();
        println!("📤 发送 Book Ticker 订阅请求: {}", subscribe_text);
        
        let msg = WsMessage::Text(subscribe_text);
        write.send(msg).await?;

        // 在独立的异步任务中处理WebSocket消息
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(_text)) => {
                        // 对于文本消息，我们只记录日志，不发送到通道
                    }
                    Ok(WsMessage::Binary(data)) => {
                        // 尝试解析 MEXC 官方 protobuf 结构
                        match PushDataV3ApiWrapper::decode(&*data) {
                            Ok(wrapper) => {
                                if let Some(_book_ticker) = wrapper.extract_book_ticker_data() {
                                    // 直接发送 wrapper 到通道，不打印任何信息
                                    if let Err(e) = tx_clone.send(wrapper).await {
                                        eprintln!("❌ 发送 Book Ticker 数据到通道失败: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("❌ 解析 Book Ticker protobuf 失败: {}", e);
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        eprintln!("❌ MEXC Book Ticker WebSocket connection closed");
                        break;
                    }
                    Ok(WsMessage::Ping(data)) => {
                        println!("🏓 收到 Ping，发送 Pong 响应");
                        let pong_msg = WsMessage::Pong(data);
                        if let Err(e) = write.send(pong_msg).await {
                            eprintln!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Ok(WsMessage::Pong(_)) => {
                        println!("🏓 收到 Pong");
                    }
                    _ => {}
                }
            }
        });

        Ok(rx)
    }

    /// 订阅深度数据 (Partial Book Depth Streams)
    ///
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "BTCUSDT" (必须大写)
    /// * `level` - 深度级别，可以是 5, 10, 或 20
    /// * `tx` - 消息发送通道
    pub async fn subscribe_depth(
        &self,
        symbol: &str,
        level: u32,
        tx: mpsc::UnboundedSender<crate::dto::mexc::PushDataV3ApiWrapper>,
    ) -> anyhow::Result<()> {
        let ws_url = self.base_url.clone();
        println!("Connecting to MEXC WebSocket for depth: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ MEXC WebSocket connected successfully for depth");

        let (mut write, mut read) = ws_stream.split();

        // 发送订阅请求
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIPTION",
            "params": [
                format!("spot@public.limit.depth.v3.api.pb@{}@{}", symbol.to_uppercase(), level)
            ]
        });

        let subscribe_text = subscribe_msg.to_string();
        println!("📤 发送深度订阅请求: {}", subscribe_text);
        
        let msg = WsMessage::Text(subscribe_text);
        write.send(msg).await?;

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                WsMessage::Text(text) => {
                    println!("📥 收到深度文本消息: {}", text);
                    
                    // 检查是否是订阅响应
                    if let Ok(response) = serde_json::from_str::<serde_json::Value>(&text) {
                        if let Some(method) = response.get("method") {
                            if method == "SUBSCRIPTION" {
                                println!("✅ 深度订阅成功: {}", text);
                            }
                        }
                    }
                }
                WsMessage::Binary(data) => {
                    println!("📊 收到深度二进制数据(protobuf)，长度: {}", data.len());
                    
                    // 尝试解析 MEXC 官方 protobuf 结构
                    match PushDataV3ApiWrapper::decode(&*data) {
                        Ok(wrapper) => {
                            println!("✅ 深度解析成功: {} | 消息类型: {}", 
                                wrapper.channel, wrapper.get_message_type());
                            
                            if let Some(depth) = wrapper.extract_limit_depth_data() {
                                println!("📊 深度数据: 买单数量: {} | 卖单数量: {} | 版本: {}", 
                                    depth.bids.len(), depth.asks.len(), depth.version);
                                
                                // 直接发送 protobuf 结构体到通道
                                if let Err(e) = tx.send(wrapper) {
                                    eprintln!("Failed to send depth message: {}", e);
                                    break;
                                }
                            } else {
                                println!("ℹ️ 未找到深度数据，消息类型: {}", wrapper.get_message_type());
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ 深度数据 Protobuf 解析失败: {}", e);
                            // 解析失败时，创建一个空的 wrapper 作为错误标记
                            // 或者可以选择跳过这个消息
                            println!("⚠️ 跳过无法解析的深度数据");
                        }
                    }
                }
                WsMessage::Close(_) => {
                    eprintln!("❌ MEXC WebSocket connection closed");
                    break;
                }
                WsMessage::Ping(data) => {
                    println!("🏓 收到 Ping，发送 Pong 响应");
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

    /// 订阅多个交易对的深度数据
    pub async fn subscribe_multiple_depths(
        &self,
        subscriptions: Vec<(String, u32)>, // (symbol, level)
        tx: mpsc::UnboundedSender<crate::dto::mexc::PushDataV3ApiWrapper>,
    ) -> anyhow::Result<()> {
        let ws_url = self.base_url.clone();
        println!("Connecting to MEXC WebSocket for multiple depth subscriptions: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ MEXC WebSocket connected successfully for multiple depth subscriptions");

        let (mut write, mut read) = ws_stream.split();

        // 发送多个订阅请求
        for (symbol, level) in &subscriptions {
            let subscribe_msg = serde_json::json!({
                "method": "SUBSCRIPTION",
                "params": [
                    format!("spot@public.limit.depth.v3.api.pb@{}@{}", symbol.to_uppercase(), level)
                ]
            });

            let subscribe_text = subscribe_msg.to_string();
            println!("📤 发送深度订阅请求: {} - Level {}", symbol, level);
            
            let msg = WsMessage::Text(subscribe_text);
            write.send(msg).await?;
            
            // 短暂延迟，避免请求过快
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                WsMessage::Text(text) => {
                    println!("📥 收到多深度文本消息: {}", text);
                    
                    // 跳过文本消息，只处理 protobuf 数据
                    println!("⚠️ 跳过文本消息，只处理 protobuf 数据");
                }
                WsMessage::Binary(data) => {
                    println!("📊 收到多深度二进制数据(protobuf)，长度: {}", data.len());
                    
                    // 尝试解析 MEXC 官方 protobuf 结构
                    match PushDataV3ApiWrapper::decode(&*data) {
                        Ok(wrapper) => {
                            if let Some(depth) = wrapper.extract_limit_depth_data() {
                                println!("📊 收到多深度数据: {} | 买单: {} | 卖单: {} | 版本: {}", 
                                    wrapper.channel, depth.bids.len(), depth.asks.len(), depth.version);
                                
                                // 直接发送 protobuf 结构体到通道
                                if let Err(e) = tx.send(wrapper) {
                                    eprintln!("Failed to send depth message: {}", e);
                                    break;
                                }
                            } else {
                                println!("ℹ️ 未找到多深度数据，消息类型: {}", wrapper.get_message_type());
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ 多深度数据 Protobuf 解析失败: {}", e);
                            // 解析失败时跳过这个消息
                            println!("⚠️ 跳过无法解析的多深度数据");
                        }
                    }
                }
                WsMessage::Close(_) => {
                    eprintln!("❌ MEXC WebSocket connection closed");
                    break;
                }
                WsMessage::Ping(data) => {
                    println!("🏓 收到 Ping，发送 Pong 响应");
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

    /// 订阅多个交易对的 Book Ticker 数据
    pub async fn subscribe_multiple_book_tickers(
        &mut self,
        symbols: Vec<String>,
        interval: &str,
    ) -> anyhow::Result<mpsc::Receiver<PushDataV3ApiWrapper>> {
        let (tx, rx) = mpsc::channel::<PushDataV3ApiWrapper>(1000);
        let ws_url = self.base_url.clone();
        println!("Connecting to MEXC Multiple Book Ticker WebSocket: {}", ws_url);

        let url: Url = Url::parse(&ws_url)?;
        let (ws_stream, _) = connect_async(url).await?;

        println!("✅ MEXC Multiple Book Ticker WebSocket connected successfully");

        let (mut write, mut read) = ws_stream.split();

        // 发送多个订阅请求
        for symbol in &symbols {
            let subscribe_msg = serde_json::json!({
                "method": "SUBSCRIPTION",
                "params": [
                    format!("spot@public.aggre.bookTicker.v3.api.pb@{}@{}", interval, symbol.to_uppercase())
                ]
            });

            let subscribe_text = subscribe_msg.to_string();
            println!("📤 发送 Book Ticker 订阅请求: {} -> {}", symbol, subscribe_text);
            
            let msg = WsMessage::Text(subscribe_text);
            write.send(msg).await?;
            
            // 短暂延迟，避免发送过快
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        // 处理接收到的消息
        while let Some(msg) = read.next().await {
            match msg? {
                WsMessage::Text(text) => {
                    println!("📥 收到多 Book Ticker 文本消息: {}", text);
                    
                    // 对于文本消息，我们只记录日志，不发送到通道
                    // 因为通道现在只接受 Book Ticker 结构体
                    println!("📝 收到多 Book Ticker 文本消息，跳过发送到通道");
                }
                WsMessage::Binary(data) => {
                    println!("📊 收到多 Book Ticker 二进制数据(protobuf)，长度: {}", data.len());
                    
                    // 尝试解析 MEXC 官方 protobuf 结构
                    match PushDataV3ApiWrapper::decode(&*data) {
                        Ok(wrapper) => {
                            if let Some(_book_ticker) = wrapper.extract_book_ticker_data() {
                                println!("📊 收到多 Book Ticker 数据: {} | 价差: {:.8} | 中间价: {:.8}", 
                                    wrapper.channel, wrapper.spread(), wrapper.mid_price());
                                
                                // 直接发送 Book Ticker 结构体到通道
                                if let Err(e) = tx.send(wrapper).await {
                                    eprintln!("Failed to send book ticker: {}", e);
                                    break;
                                }
                            } else {
                                eprintln!("⚠️  多 Book Ticker 数据为空");
                                eprintln!("🔍 调试信息 - Channel: {}, Symbol: {:?}, SendTime: {:?}", 
                                    wrapper.channel, wrapper.symbol, wrapper.send_time);
                                
                                // 对于空数据，我们只记录日志，不发送到通道
                                // 因为通道现在只接受 Book Ticker 结构体
                                println!("📝 多 Book Ticker 数据为空，跳过发送到通道");
                            }
                        }
                        Err(e) => {
                            eprintln!("❌ 多 Book Ticker protobuf 解析失败: {}", e);
                            eprintln!("🔍 原始数据长度: {} 字节", data.len());
                            
                            // 对于解析失败的数据，我们只记录日志，不发送到通道
                            // 因为通道现在只接受 Book Ticker 结构体
                            println!("📝 多 Book Ticker Protobuf 解析失败，跳过发送到通道");
                        }
                    }
                }
                WsMessage::Close(_) => {
                    eprintln!("❌ MEXC Multiple Book Ticker WebSocket connection closed");
                    break;
                }
                WsMessage::Ping(data) => {
                    println!("🏓 收到 Ping，发送 Pong 响应");
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

        Ok(rx)
    }

    /// 带重连机制的 K线订阅
    pub async fn subscribe_kline_with_reconnect(
        &self,
        symbol: &str,
        interval: &str,
        tx: mpsc::UnboundedSender<String>,
        max_retries: usize,
        retry_delay: Duration,
    ) -> anyhow::Result<()> {
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
    ) -> anyhow::Result<()> {
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
                    eprintln!("❌ MEXC WebSocket connection closed");
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

    #[tokio::test]
    async fn test_book_ticker_subscription() {
        let mut ws = MexcWebSocket::new();

        // 启动 Book Ticker WebSocket 连接
        let symbol = "BTCUSDT";
        let interval = "100ms";

        let mut rx = ws.subscribe_book_ticker(symbol, interval).await.unwrap();

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 5;

        while let Some(data) = rx.recv().await {
            println!("Received Book Ticker: {:?}", data);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }
    }

    #[tokio::test]
    async fn test_depth_subscription() {
        let ws = MexcWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel::<crate::dto::mexc::PushDataV3ApiWrapper>();

        // 启动深度 WebSocket 连接
        let symbol = "BTCUSDT";
        let level = 5;

        let ws_handle = tokio::spawn(async move { 
            ws.subscribe_depth(symbol, level, tx).await 
        });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 5;

        while let Some(data) = rx.recv().await {
            println!("Received Depth: {:?}", data);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }

    #[tokio::test]
    async fn test_multiple_depths_subscription() {
        let ws = MexcWebSocket::new();
        let (tx, mut rx) = mpsc::unbounded_channel::<crate::dto::mexc::PushDataV3ApiWrapper>();

        // 订阅多个交易对的深度数据
        let subscriptions = vec![
            ("BTCUSDT".to_string(), 5),
            ("ETHUSDT".to_string(), 10),
        ];

        let ws_handle = tokio::spawn(async move { 
            ws.subscribe_multiple_depths(subscriptions, tx).await 
        });

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 10;

        while let Some(data) = rx.recv().await {
            println!("Received Multiple Depth: {:?}", data);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }

        // 等待 WebSocket 任务完成
        let _ = ws_handle.await;
    }

    #[tokio::test]
    async fn test_multiple_book_tickers_subscription() {
        let mut ws = MexcWebSocket::new();

        // 订阅多个交易对的 Book Ticker
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let interval = "100ms";

        let mut rx = ws.subscribe_multiple_book_tickers(symbols, interval).await.unwrap();

        // 接收几条消息
        let mut message_count = 0;
        let max_messages = 10;

        while let Some(data) = rx.recv().await {
            println!("Received Multiple Book Ticker: {:?}", data);
            message_count += 1;

            if message_count >= max_messages {
                break;
            }
        }
    }
}
