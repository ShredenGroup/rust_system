use crate::common::utils::generate_hmac_signature;
use crate::dto::mexc::rest_api::{MexcOrderRequest, MexcOrderResponse, MexcOrderSide, MexcOrderType};
use anyhow::Result;
use reqwest::Client;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// MEXC 现货 API 客户端
#[derive(Debug, Clone)]
pub struct MexcSpotApi {
    pub base_url: String,
    client: Client,
    api_key: String,
    secret_key: String,
}

impl MexcSpotApi {
    /// 创建新的 MEXC 现货 API 客户端
    pub fn new(api_key: String, secret_key: String) -> Self {
        Self {
            base_url: "https://api.mexc.com".to_string(),
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

    /// 构建查询字符串（按字母顺序排序）
    pub fn build_query_string(&self, params: &HashMap<String, String>) -> String {
        let mut pairs: Vec<String> = params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        pairs.sort(); // MEXC 要求参数按字母顺序排序
        pairs.join("&")
    }

    /// 生成签名
    pub fn generate_signature(&self, query_string: &str) -> String {
        generate_hmac_signature(query_string, &self.secret_key)
    }

    /// 下单
    ///
    /// # Arguments
    /// * `request` - 订单请求
    ///
    /// # Returns
    /// * `Result<MexcOrderResponse>` - 订单响应
    ///
    /// # Example
    /// ```rust
    /// let request = MexcOrderRequest {
    ///     symbol: "MXUSDT".to_string(),
    ///     side: MexcOrderSide::Buy,
    ///     order_type: MexcOrderType::Limit,
    ///     quantity: Some("50".to_string()),
    ///     price: Some("0.1".to_string()),
    ///     timestamp: Some(MexcSpotApi::get_timestamp()),
    ///     recv_window: Some(60000),
    ///     ..Default::default()
    /// };
    ///
    /// let response = api.new_order(request).await?;
    /// ```
    pub async fn new_order(&self, request: MexcOrderRequest) -> Result<MexcOrderResponse> {
        // 构建请求参数
        let params = request.to_params();

        // 构建查询字符串
        let query_string = self.build_query_string(&params);

        // 生成签名
        let signature = self.generate_signature(&query_string);

        // 构建完整 URL
        let url = format!(
            "{}/api/v3/order?{}&signature={}",
            self.base_url, query_string, signature
        );

        // 发送请求
        let response = self
            .client
            .post(&url)
            .header("X-MEXC-APIKEY", &self.api_key)
            .send()
            .await?;

        // 检查响应状态
        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("API 请求失败: {}", error_text));
        }

        // 解析响应
        let order_response: MexcOrderResponse = response.json().await?;
        Ok(order_response)
    }

    /// 创建限价买单的便捷方法
    pub async fn limit_buy(
        &self,
        symbol: &str,
        quantity: &str,
        price: &str,
    ) -> Result<MexcOrderResponse> {
        let request = MexcOrderRequest {
            symbol: symbol.to_string(),
            side: MexcOrderSide::Buy,
            order_type: MexcOrderType::Limit,
            quantity: Some(quantity.to_string()),
            price: Some(price.to_string()),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            new_client_order_id: None,
            quote_order_qty: None,
        };

        self.new_order(request).await
    }

    /// 创建限价卖单的便捷方法
    pub async fn limit_sell(
        &self,
        symbol: &str,
        quantity: &str,
        price: &str,
    ) -> Result<MexcOrderResponse> {
        let request = MexcOrderRequest {
            symbol: symbol.to_string(),
            side: MexcOrderSide::Sell,
            order_type: MexcOrderType::Limit,
            quantity: Some(quantity.to_string()),
            price: Some(price.to_string()),
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            new_client_order_id: None,
            quote_order_qty: None,
        };

        self.new_order(request).await
    }

    /// 创建市价买单的便捷方法（使用 quantity）
    pub async fn market_buy(&self, symbol: &str, quantity: &str) -> Result<MexcOrderResponse> {
        let request = MexcOrderRequest {
            symbol: symbol.to_string(),
            side: MexcOrderSide::Buy,
            order_type: MexcOrderType::Market,
            quantity: Some(quantity.to_string()),
            price: None,
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            new_client_order_id: None,
            quote_order_qty: None,
        };

        self.new_order(request).await
    }

    /// 创建市价买单的便捷方法（使用 quoteOrderQty）
    pub async fn market_buy_with_quote_qty(
        &self,
        symbol: &str,
        quote_order_qty: &str,
    ) -> Result<MexcOrderResponse> {
        let request = MexcOrderRequest {
            symbol: symbol.to_string(),
            side: MexcOrderSide::Buy,
            order_type: MexcOrderType::Market,
            quantity: None,
            quote_order_qty: Some(quote_order_qty.to_string()),
            price: None,
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            new_client_order_id: None,
        };

        self.new_order(request).await
    }

    /// 创建市价卖单的便捷方法
    pub async fn market_sell(&self, symbol: &str, quantity: &str) -> Result<MexcOrderResponse> {
        let request = MexcOrderRequest {
            symbol: symbol.to_string(),
            side: MexcOrderSide::Sell,
            order_type: MexcOrderType::Market,
            quantity: Some(quantity.to_string()),
            price: None,
            timestamp: Some(Self::get_timestamp()),
            recv_window: Some(60000),
            new_client_order_id: None,
            quote_order_qty: None,
        };

        self.new_order(request).await
    }

    /// 创建 User Data Stream listenKey
    ///
    /// # Returns
    /// * `Result<String>` - listenKey
    ///
    /// # Example
    /// ```rust
    /// let listen_key = api.create_user_data_stream().await?;
    /// ```
    pub async fn create_user_data_stream(&self) -> Result<String> {
        let timestamp = Self::get_timestamp();
        let mut params = HashMap::new();
        params.insert("timestamp".to_string(), timestamp.to_string());
        params.insert("recvWindow".to_string(), "60000".to_string());

        let query_string = self.build_query_string(&params);
        let signature = self.generate_signature(&query_string);

        let url = format!(
            "{}/api/v3/userDataStream?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .post(&url)
            .header("X-MEXC-APIKEY", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("创建 listenKey 失败: {}", error_text));
        }

        let result: crate::dto::mexc::rest_api::MexcListenKeyResponse = response.json().await?;
        Ok(result.listen_key)
    }

    /// 更新 User Data Stream listenKey 有效期（延长 60 分钟）
    ///
    /// # Arguments
    /// * `listen_key` - 要更新的 listenKey
    ///
    /// # Returns
    /// * `Result<()>` - 成功返回 Ok(())
    ///
    /// # Example
    /// ```rust
    /// api.update_user_data_stream(&listen_key).await?;
    /// ```
    pub async fn update_user_data_stream(&self, listen_key: &str) -> Result<()> {
        let timestamp = Self::get_timestamp();
        let mut params = HashMap::new();
        params.insert("listenKey".to_string(), listen_key.to_string());
        params.insert("timestamp".to_string(), timestamp.to_string());
        params.insert("recvWindow".to_string(), "60000".to_string());

        let query_string = self.build_query_string(&params);
        let signature = self.generate_signature(&query_string);

        let url = format!(
            "{}/api/v3/userDataStream?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .put(&url)
            .header("X-MEXC-APIKEY", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("更新 listenKey 失败: {}", error_text));
        }

        Ok(())
    }

    /// 删除 User Data Stream listenKey（关闭流）
    ///
    /// # Arguments
    /// * `listen_key` - 要删除的 listenKey
    ///
    /// # Returns
    /// * `Result<()>` - 成功返回 Ok(())
    ///
    /// # Example
    /// ```rust
    /// api.delete_user_data_stream(&listen_key).await?;
    /// ```
    pub async fn delete_user_data_stream(&self, listen_key: &str) -> Result<()> {
        let timestamp = Self::get_timestamp();
        let mut params = HashMap::new();
        params.insert("listenKey".to_string(), listen_key.to_string());
        params.insert("timestamp".to_string(), timestamp.to_string());
        params.insert("recvWindow".to_string(), "60000".to_string());

        let query_string = self.build_query_string(&params);
        let signature = self.generate_signature(&query_string);

        let url = format!(
            "{}/api/v3/userDataStream?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .delete(&url)
            .header("X-MEXC-APIKEY", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("删除 listenKey 失败: {}", error_text));
        }

        Ok(())
    }

    /// 获取所有有效的 listenKey
    ///
    /// # Returns
    /// * `Result<Vec<String>>` - listenKey 列表
    ///
    /// # Example
    /// ```rust
    /// let listen_keys = api.get_user_data_streams().await?;
    /// ```
    pub async fn get_user_data_streams(&self) -> Result<Vec<String>> {
        let timestamp = Self::get_timestamp();
        let mut params = HashMap::new();
        params.insert("timestamp".to_string(), timestamp.to_string());
        params.insert("recvWindow".to_string(), "60000".to_string());

        let query_string = self.build_query_string(&params);
        let signature = self.generate_signature(&query_string);

        let url = format!(
            "{}/api/v3/userDataStream?{}&signature={}",
            self.base_url, query_string, signature
        );

        let response = self
            .client
            .get(&url)
            .header("X-MEXC-APIKEY", &self.api_key)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow::anyhow!("获取 listenKey 列表失败: {}", error_text));
        }

        let result: crate::dto::mexc::rest_api::MexcListenKeysResponse = response.json().await?;
        Ok(result.listen_keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_string_building() {
        let api = MexcSpotApi::new("test_key".to_string(), "test_secret".to_string());
        
        let mut params = HashMap::new();
        params.insert("symbol".to_string(), "MXUSDT".to_string());
        params.insert("side".to_string(), "BUY".to_string());
        params.insert("type".to_string(), "LIMIT".to_string());
        params.insert("quantity".to_string(), "50".to_string());
        params.insert("price".to_string(), "0.1".to_string());
        params.insert("timestamp".to_string(), "1666676533741".to_string());
        
        let query_string = api.build_query_string(&params);
        
        // 验证参数按字母顺序排序
        assert!(query_string.contains("price=0.1"));
        assert!(query_string.contains("quantity=50"));
        assert!(query_string.contains("side=BUY"));
        assert!(query_string.contains("symbol=MXUSDT"));
        assert!(query_string.contains("timestamp=1666676533741"));
        assert!(query_string.contains("type=LIMIT"));
    }
}

