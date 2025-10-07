use crate::common::consts::BINANCE_FUTURES_URL;
use crate::common::utils::generate_hmac_signature;
use crate::models::{TradingSignal, Signal, MarketSignal, Side};
use crate::dto::binance::rest_api::{
    OrderType, OrderSide, TimeInForce, KlineRequest, KlineResponse,
    OrderRequest, OrderResponse, BatchOrderResponseItem, BatchOrderResult
};
use anyhow::Result;
use reqwest::Client;
use serde_json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// 导入日志宏
use crate::{order_log, error_log};

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
        let response = self.client.get(&url).send().await?;
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API request failed: {}", error_text));
        }
        
        let mut klines: KlineResponse = response.json().await?;
        
        // 为每个 KlineData 设置 symbol
        let trading_symbol = crate::common::TradingSymbol::from_string(request.symbol.clone());
        for kline in &mut klines {
            kline.symbol = trading_symbol.clone();
        }
        
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
            Signal::Limit(_limit_signal) => {
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
            // 平仓操作：先取消该交易对的所有开放订单
            order_log!(info, "🔄 平仓操作：先取消 {} 的所有开放订单", signal.symbol);
            let cancel_result = self.cancel_all_open_orders(&signal.symbol, None).await;
            if cancel_result.is_ok() {
                order_log!(info, "✅ 成功取消 {} 的所有开放订单", signal.symbol);
            } else {
                // 如果取消订单失败，记录警告但继续执行平仓
                let error = cancel_result.unwrap_err();
                error_log!(warn, "⚠️ 取消开放订单失败: {}，继续执行平仓", error);
            }
            
            // 平仓操作：使用信号携带的数量，并设置 reduce_only
            let close_order_request = OrderRequest {
                symbol: signal.symbol.clone(),
                side: match signal.side {
                    Side::Buy => OrderSide::Buy,   // 平仓买入（平空仓）
                    Side::Sell => OrderSide::Sell, // 平仓卖出（平多仓）
                },
                order_type: OrderType::Market,
                quantity: Some(signal.quantity.to_string()),
                reduce_only: Some("true".to_string()),   // 必须是减仓单
                timestamp: Some(Self::get_timestamp()),
                recv_window: Some(60000),
                ..Default::default()
            };
            
            all_orders.push(close_order_request);
            
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
                
            } else {
            }
        }

        // 3. 一次性下所有订单（带重试机制）
        let batch_result = self.batch_orders_with_retry(all_orders, None).await?;
        
        // 4. 处理批量订单结果
        if batch_result.is_all_failed() {
            // 所有订单都失败了
            let first_error = &batch_result.failed_orders[0].1;
            return Err(anyhow::anyhow!("所有订单都失败了: {}", first_error.msg));
        }
        
        // 5. 收集成功的订单ID
        let mut order_ids = Vec::new();
        for order in &batch_result.successful_orders {
            order_ids.push(order.order_id.to_string());
        }
        
        // 6. 如果有部分失败的订单，记录警告
        if batch_result.is_partial_success() {
            order_log!(warn, "⚠️ 部分订单失败: 成功{}/{}，失败{}/{}", 
                batch_result.success_count(), batch_result.total_requested,
                batch_result.failure_count(), batch_result.total_requested);
            
            // 记录失败的订单详情
            for (index, error) in &batch_result.failed_orders {
                order_log!(error, "❌ 订单{}失败: 错误码={}, 消息={}", index + 1, error.code, error.msg);
            }
        }
        
        Ok(order_ids)
    }

    /// 带重试机制的批量下单（简化版）
    /// 
    /// # Arguments
    /// * `orders` - 订单列表，最多5个订单
    /// * `recv_window` - 接收窗口时间（可选）
    /// 
    /// # Returns
    /// * `Result<BatchOrderResult>` - 批量订单结果
    /// 
    /// # 重试策略
    /// - 只有错误码为1008的订单才会重试
    /// - 最多重试3次
    /// - 其他错误码的订单不会重试
    /// - 如果所有重试都失败，触发熔断机制
    pub async fn batch_orders_with_retry(
        &self,
        orders: Vec<OrderRequest>,
        recv_window: Option<u64>,
    ) -> Result<BatchOrderResult> {
        let mut result = self.batch_orders(orders.clone(), recv_window).await?;
        
        // 如果有失败的订单，检查是否需要重试（只有错误码1008才重试）
        if !result.failed_orders.is_empty() {
            // 检查是否有错误码为1008的失败订单
            let mut retryable_indices: Vec<usize> = result.failed_orders
                .iter()
                .filter(|(_, error)| error.code == 1008)
                .map(|(index, _)| *index)
                .collect();
            
            if !retryable_indices.is_empty() {
                order_log!(warn, "🔄 开始重试失败的订单: {}/{} 个订单需要重试（错误码1008）", 
                    retryable_indices.len(), result.failed_orders.len());
                
                // 重试3次
                for retry_attempt in 1..=3 {
                    if retryable_indices.is_empty() {
                        break; // 没有需要重试的订单了，停止重试
                    }
                    
                    order_log!(info, "🔄 第{}次重试: {} 个失败订单（错误码1008）", retry_attempt, retryable_indices.len());
                    
                    // 准备重试的订单（只重试错误码1008的订单）
                    let mut retry_orders = Vec::new();
                    let mut retry_indices = Vec::new();
                    
                    for original_index in &retryable_indices {
                        if let Some(order) = orders.get(*original_index) {
                            retry_orders.push(order.clone());
                            retry_indices.push(*original_index);
                        }
                    }
                    
                    // 执行重试
                    let retry_result = self.batch_orders(retry_orders, recv_window).await;
                    
                    match retry_result {
                        Ok(retry_result) => {
                            // 先计算统计信息
                            let success_count = retry_result.successful_orders.len();
                            let failure_count = retry_result.failed_orders.len();
                            
                            // 处理重试结果
                            for order in retry_result.successful_orders {
                                result.add_success(order);
                            }
                            
                            // 更新重试列表：移除成功的订单
                            let mut new_retryable_indices = Vec::new();
                            for (retry_index, original_index) in retry_indices.iter().enumerate() {
                                if retry_index < success_count {
                                    // 这个订单成功了，从重试列表中移除
                                    continue;
                                } else {
                                    // 这个订单仍然失败，保留在重试列表中
                                    new_retryable_indices.push(*original_index);
                                }
                            }
                            retryable_indices = new_retryable_indices;
                            
                            // 添加新的失败订单到结果中
                            for (original_index, error) in &retry_result.failed_orders {
                                result.add_failure(*original_index, error.clone());
                            }
                            
                            order_log!(info, "✅ 第{}次重试完成: 成功{}, 失败{}", 
                                retry_attempt, success_count, failure_count);
                        }
                        Err(e) => {
                            order_log!(error, "💥 第{}次重试失败: {}", retry_attempt, e);
                            
                            // 检查是否应该触发熔断
                            if retry_attempt == 3 {
                                order_log!(error, "🚨 触发熔断机制: 重试3次都失败，关闭系统");
                                
                                // 触发熔断，关闭整个进程
                                std::process::exit(1);
                            }
                        }
                    }
                }
            } else {
                order_log!(info, "ℹ️ 没有需要重试的订单（错误码1008），其他错误码不进行重试");
            }
        }
        
        // 最终结果统计
        order_log!(info, "📊 批量订单最终结果: 成功{}/{}，失败{}/{}", 
            result.success_count(), result.total_requested,
            result.failure_count(), result.total_requested);
        
        Ok(result)
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
    ) -> Result<BatchOrderResult> {
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
            // 记录到订单日志
            order_log!(error, "❌ 批量下单失败: HTTP状态={}, 响应={}", status, error_text);
            return Err(anyhow::anyhow!("批量下单API请求失败: HTTP状态: {}, 错误: {}", status, error_text));
        }

        // 获取响应文本进行调试
        let response_text = response.text().await?;
        // 记录到订单日志
        order_log!(info, "📡 批量下单响应: {}", response_text);

        // 解析混合响应 - 可能包含成功订单和错误
        let response_items: Vec<BatchOrderResponseItem> = serde_json::from_str(&response_text)?;
        
        // 处理混合响应
        let mut result = BatchOrderResult::new(orders.len());
        
        for (index, item) in response_items.iter().enumerate() {
            match item {
                BatchOrderResponseItem::Success(order) => {
                    result.add_success(order.clone());
                    order_log!(info, "✅ 订单{}成功: ID={}, 状态={}", index + 1, order.order_id, order.status);
                }
                BatchOrderResponseItem::Error(error) => {
                    result.add_failure(index, error.clone());
                    order_log!(error, "❌ 订单{}失败: 错误码={}, 消息={}", index + 1, error.code, error.msg);
                }
            }
        }
        
        // 记录批量订单结果摘要
        if result.is_all_success() {
            order_log!(info, "🎉 批量订单全部成功: {}/{}", result.success_count(), result.total_requested);
        } else if result.is_all_failed() {
            order_log!(error, "💥 批量订单全部失败: {}/{}", result.failure_count(), result.total_requested);
        } else if result.is_partial_success() {
            order_log!(warn, "⚠️ 批量订单部分成功: 成功{}/{}，失败{}/{}", 
                result.success_count(), result.total_requested,
                result.failure_count(), result.total_requested);
        }
        
        Ok(result)
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

        let result = self.batch_orders(orders, None).await?;
        Ok(result.successful_orders)
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

        let result = self.batch_orders(orders, None).await?;
        Ok(result.successful_orders)
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

        let result = self.batch_orders(orders, None).await?;
        Ok(result.successful_orders)
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

        let result = self.batch_orders(orders, None).await?;
        Ok(result.successful_orders)
    }

    /// 取消指定交易对的所有开放订单
    /// 
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "BTCUSDT"
    /// * `recv_window` - 接收窗口时间（可选，默认60000ms）
    /// 
    /// # Returns
    /// * `Result<()>` - 操作结果
    /// 
    /// # Example
    /// ```rust
    /// let result = api.cancel_all_open_orders("BTCUSDT", None).await?;
    /// println!("所有开放订单已取消");
    /// ```
    pub async fn cancel_all_open_orders(
        &self,
        symbol: &str,
        recv_window: Option<u64>,
    ) -> Result<()> {
        // 构建请求参数
        let mut params = HashMap::new();
        
        // 必需参数
        params.insert("symbol".to_string(), symbol.to_string());
        params.insert("timestamp".to_string(), Self::get_timestamp().to_string());
        
        // 可选参数
        if let Some(window) = recv_window {
            params.insert("recvWindow".to_string(), window.to_string());
        } else {
            params.insert("recvWindow".to_string(), "60000".to_string());
        }

        // 构建查询字符串
        let query_string = self.build_query_string(&params);

        // 生成签名
        let signature = self.generate_signature(&query_string);

        // 构建完整 URL
        let url = format!(
            "{}/allOpenOrders?{}&signature={}",
            self.base_url, query_string, signature
        );

        // 发送DELETE请求
        let response = self
            .client
            .delete(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        // 先获取状态码，因为 text() 会移动 response
        let status = response.status();

        // 检查响应状态
        if !status.is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("取消所有开放订单失败: HTTP状态: {}, 错误: {}", 
                status, error_text));
        }

        // 获取响应文本
        let response_text = response.text().await?;

        // 检查响应内容
        if response_text.contains("code") && response_text.contains("msg") {
            // 尝试解析JSON响应
            let json_result = serde_json::from_str::<serde_json::Value>(&response_text);
            if json_result.is_ok() {
                let json_response = json_result.unwrap();
                if let Some(code) = json_response.get("code") {
                    if code.as_u64() == Some(200) {
                        return Ok(());
                    } else {
                        let msg = json_response.get("msg").and_then(|m| m.as_str()).unwrap_or("未知错误");
                        return Err(anyhow::anyhow!("取消所有开放订单失败: 错误码 {}, 消息: {}", code, msg));
                    }
                }
            }
        }

        // 如果无法解析JSON，但HTTP状态是成功的，我们认为操作成功
        Ok(())
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
        
        
        // 执行批量下单
        let result = api.batch_orders(orders, None).await;
        
        if result.is_ok() {
            let batch_result = result.unwrap();
            
            // 验证订单数量
            assert_eq!(batch_result.successful_orders.len(), 3, "应该成功下3个订单");
            
            // 如果有失败的订单，记录但不失败测试
            if batch_result.is_partial_success() {
                println!("⚠️ 部分订单失败: 成功{}/{}，失败{}/{}", 
                    batch_result.success_count(), batch_result.total_requested,
                    batch_result.failure_count(), batch_result.total_requested);
            }
        } else {
            let error = result.unwrap_err();
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
        
        
        // 执行信号转订单
        let result = api.signal_to_order(&signal).await;
        
        if result.is_ok() {
            let order_ids = result.unwrap();
            
            // 验证订单数量：只有市价单，应该是1个
            assert_eq!(order_ids.len(), 1, "应该只下1个订单：主市价单");
            
        } else {
            let error = result.unwrap_err();
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
        
        
        // 执行信号转订单
        let result = api.signal_to_order(&signal).await;
        
        if result.is_ok() {
            let order_ids = result.unwrap();
            
            // 验证订单数量：市价单 + 止损单，应该是2个
            assert_eq!(order_ids.len(), 2, "应该下2个订单：主市价单 + 止损单");
            
        } else {
            let error = result.unwrap_err();
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

    #[tokio::test]
    async fn test_cancel_all_open_orders() {
        // 加载用户配置
        let user_config = load_binance_user_config().expect("Failed to load user config");
        
        let api = BinanceFuturesApi::new(user_config.api_key, user_config.secret_key);
        
        println!("🧪 开始测试取消所有开放订单功能...");
        println!("📊 测试参数:");
        println!("   交易对: TURBOUSDT");
        println!("   接收窗口: 60000ms (默认)");
        
        // 执行取消所有开放订单操作
        let result = api.cancel_all_open_orders("TURBOUSDT", None).await;
        
        if result.is_ok() {
            println!("✅ 取消所有开放订单成功！");
            println!("🎉 测试通过！成功取消TURBOUSDT的所有开放订单");
        } else {
            let error = result.unwrap_err();
            println!("❌ 取消所有开放订单失败: {}", error);
            
            // 如果失败是因为没有开放订单，这也是正常的
            if error.to_string().contains("no open orders") || 
               error.to_string().contains("no orders") {
                println!("ℹ️  没有开放订单需要取消，这也是正常情况");
                println!("🎉 测试通过！没有开放订单需要取消");
            } else {
                panic!("测试失败：{}", error);
            }
        }
    }
}

