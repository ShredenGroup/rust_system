use crate::common::consts::BINANCE_FUTURES_URL;
use crate::common::utils::generate_hmac_signature;
use crate::common::signal::{TradingSignal, Signal, MarketSignal, Side};
use crate::dto::binance::rest_api::{
    OrderType, OrderSide, TimeInForce, KlineRequest, KlineResponse,
    OrderRequest, OrderResponse
};
use anyhow::{Ok, Result};
use reqwest::Client;
use serde_json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::common::enums::{StrategyName, Exchange};

/// 币安期货 API 客户端
#[derive(Debug, Clone)]
pub struct BinanceFuturesApi {
    pub base_url: String,
    client: Client,
    api_key: String,
    secret_key: String,
}

impl BinanceFuturesApi {
    /// 创建新的币安期货 API 客户端
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self {
            base_url: BINANCE_FUTURES_URL.to_string(),
            client: Client::new(),
            api_key,
            secret_key,
        }
    }

    /// 获取当前时间戳（毫秒）
    pub fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// 构建查询字符串
    pub fn build_query_string(&self, params: &HashMap<String, String>) -> String {
        let mut pairs: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        pairs.sort(); // 币安要求参数按字母顺序排序
        pairs.join("&")
    }

    /// 生成签名
    pub fn generate_signature(&self, query_string: &str) -> String {
        generate_hmac_signature(query_string, &self.secret_key)
    }

    /// 获取K线数据
    pub async fn get_klines(&self, request: &KlineRequest) -> Result<KlineResponse> {
        let params = request.to_params()?;
        let query = self.build_query_string(&params);
        let url = format!("{}/klines?{}", self.base_url, query);
        println!("Requesting URL: {}", url);
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API request failed: {}", error_text));
        }
        
        let klines = response.json().await?;
        Ok(klines)
    }

    /// 发送下单请求
    pub async fn new_order(&self, request: OrderRequest) -> Result<OrderResponse> {
        // 构建请求参数
        let mut params = HashMap::new();

        // 必需参数 - 使用 serde 序列化
        params.insert("symbol".to_string(), request.symbol.clone());
        params.insert(
            "side".to_string(),
            serde_json::to_string(&request.side)?
                .trim_matches('"')
                .to_string(),
        );
        params.insert(
            "type".to_string(),
            serde_json::to_string(&request.order_type)?
                .trim_matches('"')
                .to_string(),
        );

        // 可选参数
        if let Some(ref position_side) = request.position_side {
            params.insert("positionSide".to_string(), position_side.clone());
        }
        if let Some(ref time_in_force) = request.time_in_force {
            params.insert(
                "timeInForce".to_string(),
                serde_json::to_string(time_in_force)?
                    .trim_matches('"')
                    .to_string(),
            );
        }
        if let Some(ref quantity) = request.quantity {
            params.insert("quantity".to_string(), quantity.clone());
        }
        if let Some(ref reduce_only) = request.reduce_only {
            params.insert("reduceOnly".to_string(), reduce_only.clone());
        }
        if let Some(ref price) = request.price {
            params.insert("price".to_string(), price.clone());
        }
        if let Some(ref new_client_order_id) = request.new_client_order_id {
            params.insert("newClientOrderId".to_string(), new_client_order_id.clone());
        }
        if let Some(ref stop_price) = request.stop_price {
            params.insert("stopPrice".to_string(), stop_price.clone());
        }
        if let Some(ref close_position) = request.close_position {
            params.insert("closePosition".to_string(), close_position.clone());
        }
        if let Some(ref activation_price) = request.activation_price {
            params.insert("activationPrice".to_string(), activation_price.clone());
        }
        if let Some(ref callback_rate) = request.callback_rate {
            params.insert("callbackRate".to_string(), callback_rate.clone());
        }
        if let Some(ref working_type) = request.working_type {
            params.insert("workingType".to_string(), working_type.clone());
        }
        if let Some(ref price_protect) = request.price_protect {
            params.insert("priceProtect".to_string(), price_protect.clone());
        }
        if let Some(ref new_order_resp_type) = request.new_order_resp_type {
            params.insert("newOrderRespType".to_string(), new_order_resp_type.clone());
        }
        if let Some(ref price_match) = request.price_match {
            params.insert("priceMatch".to_string(), price_match.clone());
        }
        if let Some(ref self_trade_prevention_mode) = request.self_trade_prevention_mode {
            params.insert(
                "selfTradePreventionMode".to_string(),
                self_trade_prevention_mode.clone(),
            );
        }
        if let Some(ref good_till_date) = request.good_till_date {
            params.insert("goodTillDate".to_string(), good_till_date.to_string());
        }

        // 添加时间戳和接收窗口
        let timestamp = request.timestamp.unwrap_or_else(Self::get_timestamp);
        params.insert("timestamp".to_string(), timestamp.to_string());
        params.insert(
            "recvWindow".to_string(),
            request.recv_window.unwrap_or(60000).to_string(),
        );

        // 构建查询字符串
        let query_string = self.build_query_string(&params);

        // 生成签名
        let signature = self.generate_signature(&query_string);

        // 构建完整 URL
        let url = format!(
            "{}/order?{}&signature={}",
            self.base_url, query_string, signature
        );

        // 发送请求 - 只使用 URL 参数，不发送 JSON body
        let response = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        // 检查响应状态
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API 请求失败: {}", error_text));
        }

        // 解析响应
        let order_response: OrderResponse = response.json().await?;
        Ok(order_response)
    }

    /// 创建市价买单的便捷方法
    pub async fn market_buy(&self, symbol: &str, quantity: &str) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            quantity: Some(quantity.to_string()),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }

    /// 创建市价卖单的便捷方法
    pub async fn market_sell(&self, symbol: &str, quantity: &str) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Market,
            quantity: Some(quantity.to_string()),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }

    /// 创建限价买单的便捷方法
    pub async fn limit_buy(
        &self,
        symbol: &str,
        quantity: &str,
        price: &str,
    ) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Limit,
            quantity: Some(quantity.to_string()),
            price: Some(price.to_string()),
            time_in_force: Some(TimeInForce::Gtc),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }

    /// 创建限价卖单的便捷方法
    pub async fn limit_sell(
        &self,
        symbol: &str,
        quantity: &str,
        price: &str,
    ) -> Result<OrderResponse> {
        let request = OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Limit,
            quantity: Some(quantity.to_string()),
            price: Some(price.to_string()),
            time_in_force: Some(TimeInForce::Gtc),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        self.new_order(request).await
    }

    /// 将交易信号转换为订单并执行
    /// 
    /// # Arguments
    /// * `signal` - 交易信号
    /// 
    /// # Returns
    /// * `Result<Vec<String>>` - 订单ID列表
    pub async fn signal_to_order(&self, signal: &TradingSignal) -> Result<Vec<String>> {
        match &signal.signal {
            Signal::Market(market_signal) => {
                // 处理市价信号
                self.mkt_sig2order(signal, market_signal).await
            }
            Signal::Limit(limit_signal) => {
                // 处理限价信号（待实现）
                Err(anyhow::anyhow!("限价信号处理功能待实现"))
            }
        }
    }

    /// 处理市价信号转换为订单
    /// 
    /// # Arguments
    /// * `signal` - 交易信号
    /// * `market_signal` - 市价信号详情
    /// 
    /// # Returns
    /// * `Result<Vec<String>>` - 订单ID列表
    async fn mkt_sig2order(&self, signal: &TradingSignal, market_signal: &MarketSignal) -> Result<Vec<String>> {
        let mut all_orders = Vec::new();
        
        // 检查是否为平仓操作
        if market_signal.is_closed {
            // 平仓操作：使用硬编码数量 10000000，并设置 reduce_only
            let close_order_request = OrderRequest {
                symbol: signal.symbol.clone(),
                side: match signal.side {
                    Side::Buy => OrderSide::Buy,   // 平仓买入（平空仓）
                    Side::Sell => OrderSide::Sell, // 平仓卖出（平多仓）
                },
                order_type: OrderType::Market,
                quantity: Some("10000000".to_string()), // 硬编码数量
                reduce_only: Some("true".to_string()),   // 必须是减仓单
                timestamp: Some(Self::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            };
            
            all_orders.push(close_order_request);
            println!("准备下1个平仓订单: 数量 10000000, reduce_only=true");
            
        } else {
            // 开仓操作：原有的逻辑
            // 1. 构建主市价单
            let main_order_request = OrderRequest {
                symbol: signal.symbol.clone(),
                side: match signal.side {
                    Side::Buy => OrderSide::Buy,
                    Side::Sell => OrderSide::Sell,
                },
                order_type: OrderType::Market,
                quantity: Some(signal.quantity.to_string()),
                timestamp: Some(Self::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            };
            
            all_orders.push(main_order_request);

            // 2. 检查是否需要添加止损止盈单
            if let (Some(stop_price), Some(profit_price)) = (market_signal.stop_price, market_signal.profit_price) {
                // 同时有止损和止盈
                
                // 构建止损单
                let stop_order_request = OrderRequest {
                    symbol: signal.symbol.clone(),
                    side: match signal.side {
                        Side::Buy => OrderSide::Sell,  // 买入后，止损是卖出
                        Side::Sell => OrderSide::Buy,  // 卖出后，止损是买入
                    },
                    order_type: OrderType::StopMarket,
                    quantity: Some(signal.quantity.to_string()),
                    stop_price: Some(stop_price.to_string()),
                    reduce_only: Some("true".to_string()),  // 止损单必须是减仓单
                    timestamp: Some(Self::get_timestamp()),
                    recv_window: Some(60000),
                    ..Default::default()
                };
                
                // 构建止盈单
                let profit_order_request = OrderRequest {
                    symbol: signal.symbol.clone(),
                    side: match signal.side {
                        Side::Buy => OrderSide::Sell,  // 买入后，止盈是卖出
                        Side::Sell => OrderSide::Buy,  // 卖出后，止盈是买入
                    },
                    order_type: OrderType::TakeProfitMarket,
                    quantity: Some(signal.quantity.to_string()),
                    stop_price: Some(profit_price.to_string()),
                    reduce_only: Some("true".to_string()),  // 止盈单必须是减仓单
                    timestamp: Some(Self::get_timestamp()),
                    recv_window: Some(60000),
                    ..Default::default()
                };
                
                all_orders.push(stop_order_request);
                all_orders.push(profit_order_request);
                
                println!("准备下3个订单: 主市价单 + 止损单 + 止盈单");
                
            } else if let Some(stop_price) = market_signal.stop_price {
                // 只有止损单
                let stop_order_request = OrderRequest {
                    symbol: signal.symbol.clone(),
                    side: match signal.side {
                        Side::Buy => OrderSide::Sell,
                        Side::Sell => OrderSide::Buy,
                    },
                    order_type: OrderType::StopMarket,
                    quantity: Some(signal.quantity.to_string()),
                    stop_price: Some(stop_price.to_string()),
                    reduce_only: Some("true".to_string()),
                    timestamp: Some(Self::get_timestamp()),
                    recv_window: Some(60000),
                    ..Default::default()
                };
                
                all_orders.push(stop_order_request);
                println!("准备下2个订单: 主市价单 + 止损单");
                
            } else if let Some(profit_price) = market_signal.profit_price {
                // 只有止盈单
                let profit_order_request = OrderRequest {
                    symbol: signal.symbol.clone(),
                    side: match signal.side {
                        Side::Buy => OrderSide::Sell,
                        Side::Sell => OrderSide::Buy,
                    },
                    order_type: OrderType::TakeProfitMarket,
                    quantity: Some(signal.quantity.to_string()),
                    stop_price: Some(profit_price.to_string()),
                    reduce_only: Some("true".to_string()),
                    timestamp: Some(Self::get_timestamp()),
                    recv_window: Some(60000),
                    ..Default::default()
                };
                
                all_orders.push(profit_order_request);
                println!("准备下2个订单: 主市价单 + 止盈单");
                
            } else {
                println!("准备下1个订单: 主市价单");
            }
        }

        // 3. 一次性下所有订单
        let responses = self.batch_orders(all_orders, None).await?;
        
        // 4. 收集所有订单ID
        let mut order_ids = Vec::new();
        for response in responses {
            order_ids.push(response.order_id.to_string());
        }
        
        if market_signal.is_closed {
            println!("平仓订单执行成功完成！共下 {} 个订单，订单ID: {:?}", order_ids.len(), order_ids);
        } else {
            println!("开仓订单执行成功完成！共下 {} 个订单，订单ID: {:?}", order_ids.len(), order_ids);
        }
        
        Ok(order_ids)
    }

    /// 批量下单 - 一次性下多个订单
    /// 
    /// # Arguments
    /// * `orders` - 订单列表，最多5个订单
    /// * `recv_window` - 接收窗口时间（可选）
    /// 
    /// # Returns
    /// * `Result<Vec<OrderResponse>>` - 订单响应列表
    /// 
    /// # Example
    /// ```rust
    /// let orders = vec![
    ///     OrderRequest {
    ///         symbol: "BTCUSDT".to_string(),
    ///         side: OrderSide::Buy,
    ///         order_type: OrderType::Limit,
    ///         quantity: Some("0.001".to_string()),
    ///         price: Some("10001".to_string()),
    ///         time_in_force: Some(TimeInForce::Gtc),
    ///         ..Default::default()
    ///     },
    ///     OrderRequest {
    ///         symbol: "BTCUSDT".to_string(),
    ///         side: OrderSide::Sell,
    ///         order_type: OrderType::Limit,
    ///         quantity: Some("0.001".to_string()),
    ///         price: Some("10002".to_string()),
    ///         time_in_force: Some(TimeInForce::Gtc),
    ///         ..Default::default()
    ///     }
    /// ];
    /// 
    /// let responses = api.batch_orders(orders, None).await?;
    /// ```
    pub async fn batch_orders(
        &self,
        orders: Vec<OrderRequest>,
        recv_window: Option<u64>,
    ) -> Result<Vec<OrderResponse>> {
        // 验证订单数量（最多5个）
        if orders.is_empty() {
            return Err(anyhow::anyhow!("订单列表不能为空"));
        }
        if orders.len() > 5 {
            return Err(anyhow::anyhow!("批量订单最多支持5个订单，当前: {}", orders.len()));
        }

        // 构建批量订单参数
        let mut params = HashMap::new();
        
        // 将订单列表转换为币安API期望的格式
        let mut binance_orders = Vec::new();
        for order in &orders {
            let mut binance_order = HashMap::new();
            binance_order.insert("symbol".to_string(), order.symbol.clone());
            binance_order.insert("side".to_string(), serde_json::to_string(&order.side)?.trim_matches('"').to_string());
            binance_order.insert("type".to_string(), serde_json::to_string(&order.order_type)?.trim_matches('"').to_string());
            
            if let Some(ref qty) = order.quantity {
                binance_order.insert("quantity".to_string(), qty.clone());
            }
            
            if let Some(ref price) = order.price {
                binance_order.insert("price".to_string(), price.clone());
            }
            
            if let Some(ref time_in_force) = order.time_in_force {
                binance_order.insert("timeInForce".to_string(), serde_json::to_string(time_in_force)?.trim_matches('"').to_string());
            }
            
            if let Some(ref stop_price) = order.stop_price {
                binance_order.insert("stopPrice".to_string(), stop_price.clone());
            }
            
            if let Some(ref reduce_only) = order.reduce_only {
                binance_order.insert("reduceOnly".to_string(), reduce_only.clone());
            }
            
            if let Some(ref position_side) = order.position_side {
                binance_order.insert("positionSide".to_string(), position_side.clone());
            }
            
            if let Some(ref new_client_order_id) = order.new_client_order_id {
                binance_order.insert("newClientOrderId".to_string(), new_client_order_id.clone());
            }
            
            if let Some(ref stop_price) = order.stop_price {
                binance_order.insert("stopPrice".to_string(), stop_price.clone());
            }
            
            if let Some(ref activation_price) = order.activation_price {
                binance_order.insert("activationPrice".to_string(), activation_price.clone());
            }
            
            if let Some(ref callback_rate) = order.callback_rate {
                binance_order.insert("callbackRate".to_string(), callback_rate.clone());
            }
            
            if let Some(ref working_type) = order.working_type {
                binance_order.insert("workingType".to_string(), serde_json::to_string(working_type)?.trim_matches('"').to_string());
            }
            
            if let Some(ref price_protect) = order.price_protect {
                binance_order.insert("priceProtect".to_string(), price_protect.clone());
            }
            
            if let Some(ref new_order_resp_type) = order.new_order_resp_type {
                binance_order.insert("newOrderRespType".to_string(), serde_json::to_string(new_order_resp_type)?.trim_matches('"').to_string());
            }
            
            if let Some(ref price_match) = order.price_match {
                binance_order.insert("priceMatch".to_string(), serde_json::to_string(price_match)?.trim_matches('"').to_string());
            }
            
            if let Some(ref self_trade_prevention_mode) = order.self_trade_prevention_mode {
                binance_order.insert("selfTradePreventionMode".to_string(), serde_json::to_string(self_trade_prevention_mode)?.trim_matches('"').to_string());
            }
            
            if let Some(ref good_till_date) = order.good_till_date {
                binance_order.insert("goodTillDate".to_string(), good_till_date.to_string());
            }
            
            binance_orders.push(binance_order);
        }
        
        // 将币安格式的订单转换为JSON字符串
        let batch_orders_json = serde_json::to_string(&binance_orders)?;
        
        // 对JSON字符串进行URL编码
        let encoded_batch_orders = urlencoding::encode(&batch_orders_json);
        params.insert("batchOrders".to_string(), encoded_batch_orders.to_string());
        
        // 添加时间戳和接收窗口
        let timestamp = Self::get_timestamp();
        params.insert("timestamp".to_string(), timestamp.to_string());
        params.insert(
            "recvWindow".to_string(),
            recv_window.unwrap_or(60000).to_string(),
        );

        // 构建查询字符串
        let query_string = self.build_query_string(&params);

        // 生成签名
        let signature = self.generate_signature(&query_string);

        // 构建完整 URL
        let url = format!(
            "{}/batchOrders?{}&signature={}",
            self.base_url, query_string, signature
        );

        println!("批量下单请求URL: {}", url);
        println!("查询字符串: {}", query_string);
        println!("签名: {}", signature);
        
        // 调试信息：显示所有参数
        println!("🔍 调试信息:");
        println!("   参数数量: {}", params.len());
        for (key, value) in &params {
            println!("   {}: {}", key, value);
        }
        
        // 调试信息：显示排序后的参数
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by(|a, b| a.0.cmp(b.0));
        println!("   排序后的参数:");
        for (key, value) in &sorted_params {
            println!("   {}: {}", key, value);
        }
        
        // 调试信息：显示用于签名的查询字符串
        let debug_query = sorted_params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        println!("   用于签名的查询字符串: {}", debug_query);
        
        // 重新生成签名进行对比
        let debug_signature = self.generate_signature(&debug_query);
        println!("   重新生成的签名: {}", debug_signature);
        println!("   原始签名: {}", signature);
        println!("   签名是否匹配: {}", signature == debug_signature);
        
        // 显示URL编码前后的对比
        println!("   JSON原始值: {}", batch_orders_json);
        println!("   JSON编码后: {}", encoded_batch_orders);
        
        // 发送请求
        let response = self
            .client
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        // 先获取状态码，因为 text() 会移动 response
        let status = response.status();
        
        // 检查响应状态
        if !status.is_success() {
            let error_text = response.text().await?;
            println!("❌ 批量下单API请求失败: HTTP状态: {}, 错误: {}", status, error_text);
            return Err(anyhow::anyhow!("批量下单API请求失败: HTTP状态: {}, 错误: {}", status, error_text));
        }

        // 获取响应文本进行调试
        let response_text = response.text().await?;
        println!("📡 API响应: {}", response_text);

        // 解析响应 - 返回订单响应列表
        let order_responses: Vec<OrderResponse> = serde_json::from_str(&response_text)?;
        
        println!("✅ 成功解析 {} 个订单响应", order_responses.len());
        for (i, order_response) in order_responses.iter().enumerate() {
            println!("   订单{}: ID={}, 状态={:?}", i + 1, order_response.order_id, order_response.status);
        }
        
        Ok(order_responses)
    }

    /// 批量市价买入的便捷方法
    pub async fn batch_market_buy(
        &self,
        symbol: &str,
        quantities: Vec<&str>,
    ) -> Result<Vec<OrderResponse>> {
        let orders: Vec<OrderRequest> = quantities
            .iter()
            .map(|qty| OrderRequest {
                symbol: symbol.to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Market,
                quantity: Some(qty.to_string()),
                timestamp: Some(Self::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            })
            .collect();

        self.batch_orders(orders, None).await
    }

    /// 批量市价卖出的便捷方法
    pub async fn batch_market_sell(
        &self,
        symbol: &str,
        quantities: Vec<&str>,
    ) -> Result<Vec<OrderResponse>> {
        let orders: Vec<OrderRequest> = quantities
            .iter()
            .map(|qty| OrderRequest {
                symbol: symbol.to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Market,
                quantity: Some(qty.to_string()),
                timestamp: Some(Self::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            })
            .collect();

        self.batch_orders(orders, None).await
    }

    /// 批量限价买入的便捷方法
    pub async fn batch_limit_buy(
        &self,
        symbol: &str,
        orders_data: Vec<(String, String)>, // (quantity, price)
    ) -> Result<Vec<OrderResponse>> {
        let orders: Vec<OrderRequest> = orders_data
            .iter()
            .map(|(qty, price)| OrderRequest {
                symbol: symbol.to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Limit,
                quantity: Some(qty.clone()),
                price: Some(price.clone()),
                time_in_force: Some(TimeInForce::Gtc),
                timestamp: Some(Self::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            })
            .collect();

        self.batch_orders(orders, None).await
    }

    /// 批量限价卖出的便捷方法
    pub async fn batch_limit_sell(
        &self,
        symbol: &str,
        orders_data: Vec<(String, String)>, // (quantity, price)
    ) -> Result<Vec<OrderResponse>> {
        let orders: Vec<OrderRequest> = orders_data
            .iter()
            .map(|(qty, price)| OrderRequest {
                symbol: symbol.to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::Limit,
                quantity: Some(qty.clone()),
                price: Some(price.clone()),
                time_in_force: Some(TimeInForce::Gtc),
                timestamp: Some(Self::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            })
            .collect();

        self.batch_orders(orders, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::config::user_config::load_binance_user_config;
    use crate::common::enums::{StrategyName, Exchange};

    #[tokio::test]
    async fn test_batch_orders_multiple_orders() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");
        
        let api = BinanceFuturesApi::new(user_config.api_key, user_config.secret_key);
        
        // 创建多个测试订单
        let orders = vec![
            OrderRequest {
                symbol: "TURBOUSDT".to_string(),
                side: OrderSide::Buy,
                order_type: OrderType::Market,
                quantity: Some("10000".to_string()),
                timestamp: Some(BinanceFuturesApi::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            },
            OrderRequest {
                symbol: "TURBOUSDT".to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::TakeProfitMarket,
                quantity: Some("10000".to_string()),
                stop_price: Some("1.0".to_string()),
                reduce_only: Some("true".to_string()),
                timestamp: Some(BinanceFuturesApi::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            },
            OrderRequest {
                symbol: "TURBOUSDT".to_string(),
                side: OrderSide::Sell,
                order_type: OrderType::StopMarket,
                quantity: Some("10000".to_string()),
                stop_price: Some("0.002".to_string()),
                reduce_only: Some("true".to_string()),
                timestamp: Some(BinanceFuturesApi::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            }
        ];
        
        println!("🧪 开始测试批量下单功能...");
        println!("📊 测试订单详情:");
        println!("   订单1: TURBOUSDT 市价买入 10000");
        println!("   订单2: TURBOUSDT 止盈卖出 10000 (价格: 1.0)");
        println!("   订单3: TURBOUSDT 止损卖出 10000 (价格: 0.002)");
        println!("   总计: {} 个订单", orders.len());
        
        // 执行批量下单
        let result = api.batch_orders(orders, None).await;
        
        if result.is_ok() {
            let responses = result.unwrap();
            println!("✅ 批量下单成功！");
            println!("📋 订单响应列表:");
            
            for (i, response) in responses.iter().enumerate() {
                println!("   订单{}: ID={}, 状态={:?}", i + 1, response.order_id, response.status);
            }
            
            println!("📊 共成功下 {} 个订单", responses.len());
            
            // 验证订单数量
            assert_eq!(responses.len(), 3, "应该成功下3个订单");
            
            println!("🎉 测试通过！成功同时下了3个订单");
        } else {
            let error = result.unwrap_err();
            println!("❌ 批量下单失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_signal_to_order_market_only() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");
        
        let api = BinanceFuturesApi::new(user_config.api_key, user_config.secret_key);
        
        // 创建测试信号：只有市价单，无止损止盈
        let signal = TradingSignal::new_market_signal(
            1,                           // id
            "TURBOUSDT".to_string(),     // symbol
            Side::Buy,                   // side: 买入
            StrategyName::MACD,          // strategy
            10000.0,                     // quantity: 10000
            Exchange::Binance,           // exchange
            0,                           // data_timestamp
            None,                        // profit_price: 无止盈
            None,                        // stop_price: 无止损
            0.5,                         // latest_price: 当前价格
        );
        
        println!("🧪 开始测试市价单信号转订单...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", signal.symbol);
        println!("   方向: {:?}", signal.side);
        println!("   数量: {}", signal.quantity);
        println!("   策略: {:?}", signal.strategy);
        println!("   无止损止盈");
        
        // 执行信号转订单
        let result = api.signal_to_order(&signal).await;
        
        if result.is_ok() {
            let order_ids = result.unwrap();
            println!("✅ 市价单信号转订单成功！");
            println!("📋 订单ID列表: {:?}", order_ids);
            println!("📊 共成功下 {} 个订单", order_ids.len());
            
            // 验证订单数量：只有市价单，应该是1个
            assert_eq!(order_ids.len(), 1, "应该只下1个订单：主市价单");
            
            println!("🎉 测试通过！成功将市价单信号转换为1个订单");
        } else {
            let error = result.unwrap_err();
            println!("❌ 市价单信号转订单失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_signal_to_order_with_stop_loss() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");
        
        let api = BinanceFuturesApi::new(user_config.api_key, user_config.secret_key);
        
        // 创建测试信号：市价单 + 止损单
        let signal = TradingSignal::new_market_signal(
            2,                           // id
            "TURBOUSDT".to_string(),     // symbol
            Side::Buy,                   // side: 买入
            StrategyName::MACD,          // strategy
            10000.0,                     // quantity: 10000
            Exchange::Binance,           // exchange
            0,                           // data_timestamp
            None,                        // profit_price: 无止盈
            Some(0.002),                 // stop_price: 0.002美金止损
            0.5,                         // latest_price: 当前价格
        );
        
        println!("🧪 开始测试市价单+止损单信号转订单...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", signal.symbol);
        println!("   方向: {:?}", signal.side);
        println!("   数量: {}", signal.quantity);
        println!("   策略: {:?}", signal.strategy);
        println!("   止损价: 0.002");
        println!("   无止盈");
        
        // 执行信号转订单
        let result = api.signal_to_order(&signal).await;
        
        if result.is_ok() {
            let order_ids = result.unwrap();
            println!("✅ 市价单+止损单信号转订单成功！");
            println!("📋 订单ID列表: {:?}", order_ids);
            println!("📊 共成功下 {} 个订单", order_ids.len());
            
            // 验证订单数量：市价单 + 止损单，应该是2个
            assert_eq!(order_ids.len(), 2, "应该下2个订单：主市价单 + 止损单");
            
            println!("🎉 测试通过！成功将市价单+止损单信号转换为2个订单");
        } else {
            let error = result.unwrap_err();
            println!("❌ 市价单+止损单信号转订单失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }

    #[tokio::test]
    async fn test_signal_to_order_close_position() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");
        
        let api = BinanceFuturesApi::new(user_config.api_key, user_config.secret_key);
        
        // 创建测试信号：平仓操作
        let signal = TradingSignal::new_market_signal(
            3,                           // id
            "TURBOUSDT".to_string(),     // symbol
            Side::Buy,                   // side: 买入平仓（平空仓）
            StrategyName::MACD,          // strategy
            10000.0,                     // quantity: 这个数量在平仓时会被忽略
            Exchange::Binance,           // exchange
            0,                           // data_timestamp
            None,                        // profit_price: 无止盈
            None,                        // stop_price: 无止损
            0.5,                         // latest_price: 当前价格
        );
        
        // 手动设置 is_closed 为 true 来测试平仓逻辑
        // 注意：这里我们需要创建一个新的 MarketSignal 来设置 is_closed
        let market_signal = MarketSignal {
            side: signal.side,
            stop_price: None,
            profit_price: None,
            is_closed: true,  // 设置为平仓模式
        };
        
        println!("🧪 开始测试平仓信号转订单...");
        println!("📊 测试信号详情:");
        println!("   交易对: {}", signal.symbol);
        println!("   方向: {:?} (平仓)", signal.side);
        println!("   策略: {:?}", signal.strategy);
        println!("   平仓模式: is_closed = true");
        println!("   硬编码数量: 10000000");
        println!("   reduce_only: true");
        
        // 直接调用 mkt_sig2order 来测试平仓逻辑
        let result = api.mkt_sig2order(&signal, &market_signal).await;
        
        if result.is_ok() {
            let order_ids = result.unwrap();
            println!("✅ 平仓信号转订单成功！");
            println!("📋 订单ID列表: {:?}", order_ids);
            println!("📊 共成功下 {} 个订单", order_ids.len());
            
            // 验证订单数量：平仓操作，应该是1个
            assert_eq!(order_ids.len(), 1, "应该下1个订单：平仓单");
            
            println!("🎉 测试通过！成功将平仓信号转换为1个订单");
        } else {
            let error = result.unwrap_err();
            println!("❌ 平仓信号转订单失败: {}", error);
            panic!("测试失败：{}", error);
        }
    }
}

