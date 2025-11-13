use serde::{Deserialize, Serialize};

/// MEXC 订单类型枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MexcOrderType {
    Limit,
    Market,
}

/// MEXC 订单方向枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MexcOrderSide {
    Buy,
    Sell,
}

/// MEXC 下单请求
#[derive(Debug, Clone)]
pub struct MexcOrderRequest {
    /// 交易对
    pub symbol: String,
    /// 订单方向
    pub side: MexcOrderSide,
    /// 订单类型
    pub order_type: MexcOrderType,
    /// 委托数量（限价单必需，市价单可选）
    pub quantity: Option<String>,
    /// 委托总额（市价单可选，与 quantity 二选一）
    pub quote_order_qty: Option<String>,
    /// 委托价格（限价单必需）
    pub price: Option<String>,
    /// 客户自定义的唯一订单ID
    pub new_client_order_id: Option<String>,
    /// 接收窗口（不能大于 60000）
    pub recv_window: Option<u64>,
    /// 时间戳
    pub timestamp: Option<u64>,
}

/// MEXC 下单响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MexcOrderResponse {
    /// 交易对
    pub symbol: String,
    /// 订单ID
    #[serde(rename = "orderId")]
    pub order_id: String,
    /// 客户端订单列表ID
    #[serde(rename = "orderListId")]
    pub order_list_id: i64,
    /// 订单价格
    pub price: String,
    /// 原始委托数量
    #[serde(rename = "origQty")]
    pub orig_qty: String,
    /// 订单类型
    #[serde(rename = "type")]
    pub order_type: String,
    /// 订单方向
    pub side: String,
    /// 下单时间
    #[serde(rename = "transactTime")]
    pub transact_time: i64,
}

impl MexcOrderRequest {
    /// 转换为查询参数字典
    pub fn to_params(&self) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();
        
        params.insert("symbol".to_string(), self.symbol.clone());
        params.insert(
            "side".to_string(),
            serde_json::to_string(&self.side)
                .unwrap()
                .trim_matches('"')
                .to_string(),
        );
        params.insert(
            "type".to_string(),
            serde_json::to_string(&self.order_type)
                .unwrap()
                .trim_matches('"')
                .to_string(),
        );
        
        if let Some(ref quantity) = self.quantity {
            params.insert("quantity".to_string(), quantity.clone());
        }
        
        if let Some(ref quote_order_qty) = self.quote_order_qty {
            params.insert("quoteOrderQty".to_string(), quote_order_qty.clone());
        }
        
        if let Some(ref price) = self.price {
            params.insert("price".to_string(), price.clone());
        }
        
        if let Some(ref new_client_order_id) = self.new_client_order_id {
            params.insert("newClientOrderId".to_string(), new_client_order_id.clone());
        }
        
        let timestamp = self.timestamp.unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });
        params.insert("timestamp".to_string(), timestamp.to_string());
        
        if let Some(recv_window) = self.recv_window {
            params.insert("recvWindow".to_string(), recv_window.to_string());
        } else {
            params.insert("recvWindow".to_string(), "60000".to_string());
        }
        
        params
    }
}

