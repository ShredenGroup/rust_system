use crate::common::consts::ASTER_FUTURES_URL;
use crate::common::utils::generate_hmac_signature;
use crate::dto::aster::rest_api::{
    OrderType, OrderSide,
    OrderRequest, OrderResponse, BatchOrderResponseItem, BatchOrderResult
};
use anyhow::Result;
use reqwest::Client;
use serde_json;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// 导入日志宏
use crate::{order_log, error_log};

/// ASTER 期货 API 客户端
#[derive(Debug, Clone)]
pub struct AsterFuturesApi {
    pub base_url: String,
    client: Client,
    api_key: String,
    secret_key: String,
}

impl AsterFuturesApi {
    /// 创建新的 ASTER 期货 API 客户端
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self {
            base_url: ASTER_FUTURES_URL.to_string(),
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
        pairs.sort(); // ASTER 要求参数按字母顺序排序
        pairs.join("&")
    }

    /// 生成签名
    pub fn generate_signature(&self, query_string: &str) -> String {
        generate_hmac_signature(query_string, &self.secret_key)
    }

    /// 批量下单
    /// 
    /// # Arguments
    /// * `orders` - 订单列表，最多5个订单
    /// * `recv_window` - 接收窗口时间（可选，默认60000ms）
    /// 
    /// # Returns
    /// * `Result<BatchOrderResult>` - 批量订单结果，包含成功和失败的订单
    /// 
    /// # Example
    /// ```
    /// let orders = vec![
    ///     OrderRequest {
    ///         symbol: "BTCUSDT".to_string(),
    ///         side: OrderSide::Buy,
    ///         order_type: OrderType::Market,
    ///         quantity: Some("0.001".to_string()),
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
        
        // 将订单列表转换为 ASTER API 期望的格式
        let mut aster_orders = Vec::new();
        for order in &orders {
            let mut aster_order = HashMap::new();
            
            // 必需参数
            aster_order.insert("symbol".to_string(), order.symbol.clone());
            aster_order.insert("side".to_string(), serde_json::to_string(&order.side)?.trim_matches('"').to_string());
            aster_order.insert("type".to_string(), serde_json::to_string(&order.order_type)?.trim_matches('"').to_string());
            
            // 可选参数
            if let Some(ref position_side) = order.position_side {
                aster_order.insert("positionSide".to_string(), position_side.clone());
            }
            
            if let Some(ref time_in_force) = order.time_in_force {
                aster_order.insert("timeInForce".to_string(), serde_json::to_string(time_in_force)?.trim_matches('"').to_string());
            }
            
            if let Some(ref quantity) = order.quantity {
                aster_order.insert("quantity".to_string(), quantity.clone());
            }
            
            if let Some(ref reduce_only) = order.reduce_only {
                aster_order.insert("reduceOnly".to_string(), reduce_only.clone());
            }
            
            if let Some(ref price) = order.price {
                aster_order.insert("price".to_string(), price.clone());
            }
            
            if let Some(ref new_client_order_id) = order.new_client_order_id {
                aster_order.insert("newClientOrderId".to_string(), new_client_order_id.clone());
            }
            
            if let Some(ref stop_price) = order.stop_price {
                aster_order.insert("stopPrice".to_string(), stop_price.clone());
            }
            
            if let Some(ref activation_price) = order.activation_price {
                aster_order.insert("activationPrice".to_string(), activation_price.clone());
            }
            
            if let Some(ref callback_rate) = order.callback_rate {
                aster_order.insert("callbackRate".to_string(), callback_rate.clone());
            }
            
            if let Some(ref working_type) = order.working_type {
                aster_order.insert("workingType".to_string(), working_type.clone());
            }
            
            if let Some(ref price_protect) = order.price_protect {
                aster_order.insert("priceProtect".to_string(), price_protect.clone());
            }
            
            if let Some(ref new_order_resp_type) = order.new_order_resp_type {
                aster_order.insert("newOrderRespType".to_string(), new_order_resp_type.clone());
            }
            
            aster_orders.push(aster_order);
        }
        
        // 将 ASTER 格式的订单转换为JSON字符串
        let batch_orders_json = serde_json::to_string(&aster_orders)?;
        
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
            "{}/fapi/v1/batchOrders?{}&signature={}",
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
            order_log!(error, "❌ ASTER 批量下单失败: HTTP状态={}, 响应={}", status, error_text);
            return Err(anyhow::anyhow!("ASTER 批量下单API请求失败: HTTP状态: {}, 错误: {}", status, error_text));
        }

        // 获取响应文本进行调试
        let response_text = response.text().await?;
        // 记录到订单日志
        order_log!(info, "📡 ASTER 批量下单响应: {}", response_text);

        // 解析混合响应 - 可能包含成功订单和错误
        let response_items: Vec<BatchOrderResponseItem> = serde_json::from_str(&response_text)?;
        
        // 处理混合响应
        let mut result = BatchOrderResult::new(orders.len());
        
        for (index, item) in response_items.iter().enumerate() {
            match item {
                BatchOrderResponseItem::Success(order_response) => {
                    result.successful_orders.push(order_response.clone());
                    order_log!(info, "✅ ASTER 订单成功 [{}]: orderId={}, symbol={}, side={}, type={}, status={}", 
                        index, order_response.order_id, order_response.symbol, order_response.side, 
                        order_response.order_type, order_response.status);
                }
                BatchOrderResponseItem::Error(error_response) => {
                    result.failed_orders.push((index, error_response.clone()));
                    error_log!(error, "❌ ASTER 订单失败 [{}]: code={}, msg={}", 
                        index, error_response.code, error_response.msg);
                }
            }
        }
        
        Ok(result)
    }

    /// 创建市价买单的便捷方法
    pub async fn market_buy(&self, symbol: &str, quantity: &str) -> Result<OrderResponse> {
        let orders = vec![OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            quantity: Some(quantity.to_string()),
            ..Default::default()
        }];

        let result = self.batch_orders(orders, None).await?;
        if let Some(order) = result.successful_orders.first() {
            Ok(order.clone())
        } else if let Some((_, error)) = result.failed_orders.first() {
            Err(anyhow::anyhow!("下单失败: code={}, msg={}", error.code, error.msg))
        } else {
            Err(anyhow::anyhow!("下单失败: 未知错误"))
        }
    }

    /// 创建市价卖单的便捷方法
    pub async fn market_sell(&self, symbol: &str, quantity: &str) -> Result<OrderResponse> {
        let orders = vec![OrderRequest {
            symbol: symbol.to_string(),
            side: OrderSide::Sell,
            order_type: OrderType::Market,
            quantity: Some(quantity.to_string()),
            ..Default::default()
        }];

        let result = self.batch_orders(orders, None).await?;
        if let Some(order) = result.successful_orders.first() {
            Ok(order.clone())
        } else if let Some((_, error)) = result.failed_orders.first() {
            Err(anyhow::anyhow!("下单失败: code={}, msg={}", error.code, error.msg))
        } else {
            Err(anyhow::anyhow!("下单失败: 未知错误"))
        }
    }

    /// 取消指定交易对的所有开放订单
    /// 
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "ASTERUSDT"
    /// * `recv_window` - 接收窗口时间（可选，默认60000ms）
    /// 
    /// # Returns
    /// * `Result<()>` - 操作结果
    /// 
    /// # Example
    /// ```rust
    /// let result = api.cancel_all_open_orders("ASTERUSDT", None).await?;
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
            "{}/fapi/v1/allOpenOrders?{}&signature={}",
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
            order_log!(error, "❌ ASTER 取消所有开放订单失败: HTTP状态={}, 响应={}", status, error_text);
            return Err(anyhow::anyhow!("取消所有开放订单失败: HTTP状态: {}, 错误: {}", 
                status, error_text));
        }

        // 获取响应文本
        let response_text = response.text().await?;
        order_log!(info, "📡 ASTER 取消所有开放订单响应: {}", response_text);

        // 检查响应内容 - ASTER 返回 {"code": "200", "msg": "..."}
        if let Ok(json_response) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if let Some(code) = json_response.get("code") {
                // code 可能是字符串 "200" 或数字 200
                let is_success = match code {
                    serde_json::Value::String(s) => s == "200",
                    serde_json::Value::Number(n) => n.as_u64() == Some(200),
                    _ => false,
                };
                
                if is_success {
                    order_log!(info, "✅ ASTER 成功取消 {} 的所有开放订单", symbol);
                    return Ok(());
                } else {
                    let msg = json_response.get("msg")
                        .and_then(|m| m.as_str())
                        .unwrap_or("未知错误");
                    return Err(anyhow::anyhow!("取消所有开放订单失败: code={:?}, msg={}", code, msg));
                }
            }
        }

        // 如果无法解析JSON，但HTTP状态是成功的，我们认为操作成功
        order_log!(info, "✅ ASTER 成功取消 {} 的所有开放订单", symbol);
        Ok(())
    }

    /// 批量取消订单
    /// 
    /// # Arguments
    /// * `symbol` - 交易对符号，如 "ASTERUSDT"
    /// * `order_id_list` - 订单ID列表（可选，最多10个）
    /// * `orig_client_order_id_list` - 客户端订单ID列表（可选，最多10个）
    /// * `recv_window` - 接收窗口时间（可选，默认60000ms）
    /// 
    /// # Returns
    /// * `Result<BatchOrderResult>` - 批量取消结果，包含成功和失败的订单
    /// 
    /// # Note
    /// orderIdList 和 origClientOrderIdList 至少要提供一个
    /// 
    /// # Example
    /// ```rust
    /// // 使用订单ID列表
    /// let result = api.cancel_batch_orders(
    ///     "ASTERUSDT",
    ///     Some(vec![1234567, 2345678]),
    ///     None,
    ///     None
    /// ).await?;
    /// 
    /// // 使用客户端订单ID列表
    /// let result = api.cancel_batch_orders(
    ///     "ASTERUSDT",
    ///     None,
    ///     Some(vec!["my_id_1".to_string(), "my_id_2".to_string()]),
    ///     None
    /// ).await?;
    /// ```
    pub async fn cancel_batch_orders(
        &self,
        symbol: &str,
        order_id_list: Option<Vec<i64>>,
        orig_client_order_id_list: Option<Vec<String>>,
        recv_window: Option<u64>,
    ) -> Result<BatchOrderResult> {
        // 验证参数：至少需要提供一个列表
        if order_id_list.is_none() && orig_client_order_id_list.is_none() {
            return Err(anyhow::anyhow!("orderIdList 和 origClientOrderIdList 至少要提供一个"));
        }

        // 验证列表长度（最多10个）
        if let Some(ref order_ids) = order_id_list {
            if order_ids.is_empty() {
                return Err(anyhow::anyhow!("orderIdList 不能为空"));
            }
            if order_ids.len() > 10 {
                return Err(anyhow::anyhow!("orderIdList 最多支持10个订单，当前: {}", order_ids.len()));
            }
        }

        if let Some(ref client_order_ids) = orig_client_order_id_list {
            if client_order_ids.is_empty() {
                return Err(anyhow::anyhow!("origClientOrderIdList 不能为空"));
            }
            if client_order_ids.len() > 10 {
                return Err(anyhow::anyhow!("origClientOrderIdList 最多支持10个订单，当前: {}", client_order_ids.len()));
            }
        }

        // 计算总订单数
        let total_orders = order_id_list.as_ref().map(|v| v.len()).unwrap_or(0) +
            orig_client_order_id_list.as_ref().map(|v| v.len()).unwrap_or(0);

        // 构建请求参数
        let mut params = HashMap::new();
        
        // 必需参数
        params.insert("symbol".to_string(), symbol.to_string());
        params.insert("timestamp".to_string(), Self::get_timestamp().to_string());
        
        // 可选参数
        if let Some(ref order_ids) = order_id_list {
            // 将订单ID列表转换为JSON数组字符串，然后URL编码
            let order_ids_json = serde_json::to_string(order_ids)?;
            let encoded_order_ids = urlencoding::encode(&order_ids_json);
            params.insert("orderIdList".to_string(), encoded_order_ids.to_string());
        }

        if let Some(ref client_order_ids) = orig_client_order_id_list {
            // 将客户端订单ID列表转换为JSON数组字符串，然后URL编码
            let client_order_ids_json = serde_json::to_string(client_order_ids)?;
            let encoded_client_order_ids = urlencoding::encode(&client_order_ids_json);
            params.insert("origClientOrderIdList".to_string(), encoded_client_order_ids.to_string());
        }

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
            "{}/fapi/v1/batchOrders?{}&signature={}",
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
            order_log!(error, "❌ ASTER 批量取消订单失败: HTTP状态={}, 响应={}", status, error_text);
            return Err(anyhow::anyhow!("批量取消订单API请求失败: HTTP状态: {}, 错误: {}", status, error_text));
        }

        // 获取响应文本进行调试
        let response_text = response.text().await?;
        order_log!(info, "📡 ASTER 批量取消订单响应: {}", response_text);

        // 解析混合响应 - 可能包含成功订单和错误
        let response_items: Vec<BatchOrderResponseItem> = serde_json::from_str(&response_text)?;
        
        // 处理混合响应
        let mut result = BatchOrderResult::new(total_orders);
        
        for (index, item) in response_items.iter().enumerate() {
            match item {
                BatchOrderResponseItem::Success(order_response) => {
                    result.successful_orders.push(order_response.clone());
                    order_log!(info, "✅ ASTER 订单取消成功 [{}]: orderId={}, symbol={}, status={}", 
                        index, order_response.order_id, order_response.symbol, order_response.status);
                }
                BatchOrderResponseItem::Error(error_response) => {
                    result.failed_orders.push((index, error_response.clone()));
                    error_log!(error, "❌ ASTER 订单取消失败 [{}]: code={}, msg={}", 
                        index, error_response.code, error_response.msg);
                }
            }
        }
        
        Ok(result)
    }
}

