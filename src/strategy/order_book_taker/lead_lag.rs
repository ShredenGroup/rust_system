use crate::dto::binance::websocket::BookTickerData as BinanceBookTickerData;
use crate::dto::aster::websocket::AsterBookTickerData;
use crate::exchange_api::aster::AsterFuturesApi;
use crate::dto::aster::rest_api::{OrderRequest, OrderSide, OrderType};
use tokio::sync::mpsc;
use std::sync::Arc;
use crate::{order_log, error_log};

/// 交易方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TradeDirection {
    Long,   // 做多
    Short,  // 做空
    None,   // 无持仓
}

/// Lead-Lag 策略
/// 基于 Binance 和 ASTER 的 bookTicker 数据进行套利分析
pub struct LeadLagStrategy {
    binance_ticker_rx: mpsc::Receiver<BinanceBookTickerData>,
    aster_ticker_rx: mpsc::Receiver<AsterBookTickerData>,
    
    // ASTER API 客户端（用于实盘交易）
    aster_api: Arc<AsterFuturesApi>,
    symbol: String,      // 交易对，如 "ASTERUSDT"
    quantity: String,    // 交易数量
    
    // 最新的 fair price（用于开仓判断）
    latest_binance_fair_price: Option<f64>,
    latest_aster_fair_price: Option<f64>,
    
    // 最新的 ASTER 订单簿价格（用于止损止盈判断）
    latest_aster_bid_price: Option<f64>,
    latest_aster_ask_price: Option<f64>,
    
    // 当前持仓状态
    current_position: TradeDirection,
    entry_price: Option<f64>, // 开仓价格（使用订单簿价格：做多用ask，做空用bid）
    open_order_ids: Vec<i64>, // 开仓时的订单ID列表（用于管理订单）
    entry_count: u64, // 开仓计数器
    
    // 策略参数
    entry_threshold: f64,  // 入场阈值 0.0003
    stop_loss: f64,       // 止损 0.0001
    take_profit: f64,     // 止盈 0.0003
    max_spread: f64,      // 最大允许价差（流动性保护）0.0001
}

impl LeadLagStrategy {
    /// 创建新的 Lead-Lag 策略实例
    pub fn new(
        binance_ticker_rx: mpsc::Receiver<BinanceBookTickerData>,
        aster_ticker_rx: mpsc::Receiver<AsterBookTickerData>,
        aster_api: Arc<AsterFuturesApi>,
        symbol: String,
        quantity: String,
    ) -> Self {
        Self {
            binance_ticker_rx,
            aster_ticker_rx,
            aster_api,
            symbol,
            quantity,
            latest_binance_fair_price: None,
            latest_aster_fair_price: None,
            latest_aster_bid_price: None,
            latest_aster_ask_price: None,
            current_position: TradeDirection::None,
            entry_price: None,
            open_order_ids: Vec::new(),
            entry_threshold: 0.0005,
            stop_loss: 0.0005,
            take_profit: 0.0005,
            max_spread: 0.0001,
            entry_count: 0,
        }
    }

    /// 计算公平价格
    /// 
    /// # Arguments
    /// * `bid_price` - 最佳买价
    /// * `bid_qty` - 最佳买量
    /// * `ask_price` - 最佳卖价
    /// * `ask_qty` - 最佳卖量
    /// 
    /// # Returns
    /// 公平价格，精确到小数点后5位
    /// 
    /// 计算公式：加权平均价格 = (bid_price * bid_qty + ask_price * ask_qty) / (bid_qty + ask_qty)
    fn calculate_fair_price(
        bid_price: f64,
        bid_qty: f64,
        ask_price: f64,
        ask_qty: f64,
    ) -> f64 {
        let bid_value = bid_price * bid_qty;
        let ask_value = ask_price * ask_qty;
        let total_qty = bid_qty + ask_qty;
        
        if total_qty > 0.0 {
            let fair_price = (bid_value + ask_value) / total_qty;
            // 精确到小数点后5位
            (fair_price * 100000.0).round() / 100000.0
        } else {
            0.0
        }
    }

    /// 检查是否有套利机会并执行交易逻辑
    async fn check_and_execute_trade(&mut self) {
        let binance_price = match self.latest_binance_fair_price {
            Some(p) => p,
            None => return,
        };
        
        // 不再使用 ASTER 的 fair price 做入场判断

        // 检查当前持仓状态
        match self.current_position {
            TradeDirection::None => {
                // 无持仓，检查开仓机会
                // 需要同时有 ASTER 的订单簿价格才能开仓
                let aster_ask = match self.latest_aster_ask_price {
                    Some(p) => p,
                    None => return, // 没有 ASTER 订单簿数据，无法开仓
                };
                
                let aster_bid = match self.latest_aster_bid_price {
                    Some(p) => p,
                    None => return,
                };
                
                // 流动性保护：检查 ASTER 的价差
                let aster_spread = aster_ask - aster_bid;
                if aster_spread > self.max_spread {
                    // 价差太大，流动性不足，不开仓
                    return;
                }
                
                // Binance fair price > ASTER ask + 阈值 -> 在 ASTER 做多（用 ask 价格开仓）
                let long_diff = binance_price - aster_ask;
                if long_diff > self.entry_threshold {
                    let stop_loss_price = format!("{:.5}", aster_bid - self.stop_loss);
                    
                    // 构建批量订单：市价买单 + 止损单
                    let orders = vec![
                        // 市价买单
                        OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Buy,
                            order_type: OrderType::Market,
                            quantity: Some(self.quantity.clone()),
                            ..Default::default()
                        },
                        // 止损单（市价卖出，触发价为 stop_loss_price）
                        OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Sell,
                            order_type: OrderType::StopMarket,
                            quantity: Some(self.quantity.clone()),
                            stop_price: Some(stop_loss_price.clone()),
                            reduce_only: Some("true".to_string()),
                            ..Default::default()
                        },
                    ];
                    
                    // 执行批量下单
                    match self.aster_api.batch_orders(orders, None).await {
                        Ok(result) => {
                            if result.is_all_success() {
                                // 保存订单ID
                                self.open_order_ids = result.successful_orders.iter()
                                    .map(|o| o.order_id)
                                    .collect();
                                
                                self.current_position = TradeDirection::Long;
                                self.entry_price = Some(aster_ask);
                                self.entry_count += 1;
                                
                                println!("🟢 【开仓】在 ASTER 做多 - 实盘下单成功");
                                println!("   开仓价格 (Ask): {:.5}", aster_ask);
                                println!("   Binance Fair Price: {:.5}", binance_price);
                                println!("   ASTER 价差: {:.5} (Bid: {:.5}, Ask: {:.5})", aster_spread, aster_bid, aster_ask);
                                println!("   价差: {:.5} (Binance Fair - ASTER Ask, 超过阈值 {:.5})", long_diff, self.entry_threshold);
                                println!("   数量: {}", self.quantity);
                                println!("   止损价格: {}", stop_loss_price);
                                println!("   止盈价格: {:.5} (Ask价格上涨 {:.5})", aster_ask + self.take_profit, self.take_profit);
                                println!("   订单ID: {:?}", self.open_order_ids);
                                println!("   当前为第 {} 次开仓", self.entry_count);
                                println!("   ────────────────────────────────────────────────────────");
                                println!();
                                
                                order_log!(info, "✅ Lead-Lag 策略开仓成功 - 做多 {} 数量: {}, 订单ID: {:?}", 
                                    self.symbol, self.quantity, self.open_order_ids);
                                order_log!(info, "📈 本次为第 {} 次开仓", self.entry_count);
                            } else {
                                error_log!(error, "❌ Lead-Lag 策略开仓失败 - 部分订单失败: 成功{}/{}, 失败{}/{}",
                                    result.successful_orders.len(), result.total_requested,
                                    result.failed_orders.len(), result.total_requested);
                                
                                // 检查是否有 -2021 错误（订单会立即触发），需要平仓
                                let mut need_close_position = false;
                                for (_, error) in &result.failed_orders {
                                    error_log!(error, "   订单失败: code={}, msg={}", error.code, error.msg);
                                    if error.code == -2021 {
                                        // 订单会立即触发，说明可能已经有仓位，需要平仓
                                        need_close_position = true;
                                    }
                                }
                                
                                // 如果检测到 -2021 错误，发出平仓请求
                                if need_close_position {
                                    error_log!(warn, "⚠️ 检测到 -2021 错误（订单会立即触发），执行紧急平仓");
                                    
                                    // 发出平仓单（做多时平仓用卖出）
                                    let close_order = OrderRequest {
                                        symbol: self.symbol.clone(),
                                        side: OrderSide::Sell,
                                        order_type: OrderType::Market,
                                        quantity: Some(self.quantity.clone()),
                                        reduce_only: Some("true".to_string()),
                                        ..Default::default()
                                    };
                                    
                                    match self.aster_api.batch_orders(vec![close_order], None).await {
                                        Ok(close_result) => {
                                            if close_result.is_all_success() {
                                                order_log!(info, "✅ 紧急平仓成功 - 订单ID: {:?}", 
                                                    close_result.successful_orders.iter().map(|o| o.order_id).collect::<Vec<_>>());
                                            } else {
                                                for (_, error) in &close_result.failed_orders {
                                                    error_log!(error, "   紧急平仓失败: code={}, msg={}", error.code, error.msg);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error_log!(error, "❌ 紧急平仓下单失败: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error_log!(error, "❌ Lead-Lag 策略开仓下单失败: {}", e);
                        }
                    }
                }
                // ASTER bid > Binance fair price + 阈值 -> 在 ASTER 做空（用 bid 价格开仓）
                else {
                    let short_diff = aster_bid - binance_price;
                    if short_diff > self.entry_threshold {
                    let stop_loss_price = format!("{:.5}", aster_ask + self.stop_loss);
                    
                    // 构建批量订单：市价卖单 + 止损单
                    let orders = vec![
                        // 市价卖单
                        OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Sell,
                            order_type: OrderType::Market,
                            quantity: Some(self.quantity.clone()),
                            ..Default::default()
                        },
                        // 止损单（市价买入，触发价为 stop_loss_price）
                        OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Buy,
                            order_type: OrderType::StopMarket,
                            quantity: Some(self.quantity.clone()),
                            stop_price: Some(stop_loss_price.clone()),
                            reduce_only: Some("true".to_string()),
                            ..Default::default()
                        },
                    ];
                    
                    // 执行批量下单
                    match self.aster_api.batch_orders(orders, None).await {
                        Ok(result) => {
                            if result.is_all_success() {
                                // 保存订单ID
                                self.open_order_ids = result.successful_orders.iter()
                                    .map(|o| o.order_id)
                                    .collect();
                                
                                self.current_position = TradeDirection::Short;
                                self.entry_price = Some(aster_bid);
                                self.entry_count += 1;
                                
                                println!("🔴 【开仓】在 ASTER 做空 - 实盘下单成功");
                                println!("   开仓价格 (Bid): {:.5}", aster_bid);
                                println!("   Binance Fair Price: {:.5}", binance_price);
                                println!("   ASTER 价差: {:.5} (Bid: {:.5}, Ask: {:.5})", aster_spread, aster_bid, aster_ask);
                                println!("   价差: {:.5} (ASTER Bid - Binance Fair, 超过阈值 {:.5})", short_diff, self.entry_threshold);
                                println!("   数量: {}", self.quantity);
                                println!("   止损价格: {}", stop_loss_price);
                                println!("   止盈价格: {:.5} (Bid价格下跌 {:.5})", aster_bid - self.take_profit, self.take_profit);
                                println!("   订单ID: {:?}", self.open_order_ids);
                                println!("   当前为第 {} 次开仓", self.entry_count);
                                println!("   ────────────────────────────────────────────────────────");
                                println!();
                                
                                order_log!(info, "✅ Lead-Lag 策略开仓成功 - 做空 {} 数量: {}, 订单ID: {:?}", 
                                    self.symbol, self.quantity, self.open_order_ids);
                                order_log!(info, "📈 本次为第 {} 次开仓", self.entry_count);
                            } else {
                                error_log!(error, "❌ Lead-Lag 策略开仓失败 - 部分订单失败: 成功{}/{}, 失败{}/{}",
                                    result.successful_orders.len(), result.total_requested,
                                    result.failed_orders.len(), result.total_requested);
                                
                                // 检查是否有 -2021 错误（订单会立即触发），需要平仓
                                let mut need_close_position = false;
                                for (_, error) in &result.failed_orders {
                                    error_log!(error, "   订单失败: code={}, msg={}", error.code, error.msg);
                                    if error.code == -2021 {
                                        // 订单会立即触发，说明可能已经有仓位，需要平仓
                                        need_close_position = true;
                                    }
                                }
                                
                                // 如果检测到 -2021 错误，发出平仓请求
                                if need_close_position {
                                    error_log!(warn, "⚠️ 检测到 -2021 错误（订单会立即触发），执行紧急平仓");
                                    
                                    // 发出平仓单（做空时平仓用买入）
                                    let close_order = OrderRequest {
                                        symbol: self.symbol.clone(),
                                        side: OrderSide::Buy,
                                        order_type: OrderType::Market,
                                        quantity: Some(self.quantity.clone()),
                                        reduce_only: Some("true".to_string()),
                                        ..Default::default()
                                    };
                                    
                                    match self.aster_api.batch_orders(vec![close_order], None).await {
                                        Ok(close_result) => {
                                            if close_result.is_all_success() {
                                                order_log!(info, "✅ 紧急平仓成功 - 订单ID: {:?}", 
                                                    close_result.successful_orders.iter().map(|o| o.order_id).collect::<Vec<_>>());
                                            } else {
                                                for (_, error) in &close_result.failed_orders {
                                                    error_log!(error, "   紧急平仓失败: code={}, msg={}", error.code, error.msg);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            error_log!(error, "❌ 紧急平仓下单失败: {}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error_log!(error, "❌ Lead-Lag 策略开仓下单失败: {}", e);
                        }
                    }
                }
            }},
            
            TradeDirection::Long => {
                // 持有多头仓位，检查止损和止盈
                // 做多时，使用 ASTER 的 ask 价格来判断（买入时用卖价）
                let current_ask = match self.latest_aster_ask_price {
                    Some(p) => p,
                    None => return, // 没有订单簿数据，无法判断
                };
                
                if let Some(entry) = self.entry_price {
                    // 计算止损价格（做多：entry_price - stop_loss）
                    let stop_loss_price = entry - self.stop_loss;
                    
                    // 止损判断：当前 ask1 价格低于止损价格
                    if current_ask <= stop_loss_price {
                        println!("⛔ 【止损平仓】多头仓位止损");
                        println!("   开仓价格 (Ask): {:.5}", entry);
                        println!("   当前价格 (Ask): {:.5}", current_ask);
                        println!("   止损价格: {:.5}", stop_loss_price);
                        println!("   价格变化: {:.5}", current_ask - entry);
                        println!("   亏损: {:.5}", entry - current_ask);
                        println!("   ────────────────────────────────────────────────────────");
                        println!();
                        
                        self.current_position = TradeDirection::None;
                        self.entry_price = None;
                        self.open_order_ids.clear();
                    }
                    // 止盈：ask 价格上涨超过 0.0003 - 主动发出止盈单并取消所有订单
                    else {
                        let price_change = current_ask - entry;
                        if price_change >= self.take_profit {
                            // 1. 先取消所有开放订单（包括止损单）
                            match self.aster_api.cancel_all_open_orders(&self.symbol, None).await {
                                Ok(_) => {
                                    order_log!(info, "✅ 止盈操作：成功取消所有开放订单");
                                }
                                Err(e) => {
                                    error_log!(warn, "⚠️ 止盈操作：取消订单失败: {}，继续执行止盈", e);
                                }
                            }
                            
                            // 2. 发出止盈单（市价卖出）
                            let take_profit_order = OrderRequest {
                            symbol: self.symbol.clone(),
                            side: OrderSide::Sell,
                            order_type: OrderType::Market,
                            quantity: Some(self.quantity.clone()),
                            reduce_only: Some("true".to_string()),
                            ..Default::default()
                        };
                        
                            match self.aster_api.batch_orders(vec![take_profit_order], None).await {
                                Ok(result) => {
                                    if result.is_all_success() {
                                        println!("✅ 【止盈平仓】多头仓位止盈 - 实盘下单成功");
                                        println!("   开仓价格 (Ask): {:.5}", entry);
                                        println!("   平仓价格 (Ask): {:.5}", current_ask);
                                        println!("   价格变化: {:.5}", price_change);
                                        println!("   盈利: {:.5}", price_change);
                                        println!("   订单ID: {:?}", result.successful_orders.iter().map(|o| o.order_id).collect::<Vec<_>>());
                                        println!("   ────────────────────────────────────────────────────────");
                                        println!();
                                        
                                        order_log!(info, "✅ Lead-Lag 策略止盈成功 - 多头平仓, 盈利: {:.5}", price_change);
                                        
                                        self.current_position = TradeDirection::None;
                                        self.entry_price = None;
                                        self.open_order_ids.clear();
                                    } else {
                                        error_log!(error, "❌ 止盈下单失败 - 部分订单失败");
                                        for (_, error) in &result.failed_orders {
                                            error_log!(error, "   订单失败: code={}, msg={}", error.code, error.msg);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error_log!(error, "❌ 止盈下单失败: {}", e);
                                }
                            }
                        }
                    }
                }
            },
            
            TradeDirection::Short => {
                // 持有空头仓位，检查止损和止盈
                // 做空时，使用 ASTER 的 bid 价格来判断（卖出时用买价）
                let current_bid = match self.latest_aster_bid_price {
                    Some(p) => p,
                    None => return, // 没有订单簿数据，无法判断
                };
                
                if let Some(entry) = self.entry_price {
                    // 计算止损价格（做空：entry_price + stop_loss）
                    let stop_loss_price = entry + self.stop_loss;
                    
                    // 止损判断：当前 bid1 价格高于止损价格
                    if current_bid >= stop_loss_price {
                        println!("⛔ 【止损平仓】空头仓位止损");
                        println!("   开仓价格 (Bid): {:.5}", entry);
                        println!("   当前价格 (Bid): {:.5}", current_bid);
                        println!("   止损价格: {:.5}", stop_loss_price);
                        println!("   价格变化: {:.5}", current_bid - entry);
                        println!("   亏损: {:.5}", current_bid - entry);
                        println!("   ────────────────────────────────────────────────────────");
                        println!();
                        
                        self.current_position = TradeDirection::None;
                        self.entry_price = None;
                        self.open_order_ids.clear();
                    }
                    // 止盈：bid 价格下跌超过 0.0003（对空头有利）- 主动发出止盈单并取消所有订单
                    else {
                        let price_change = entry - current_bid; // 做空：价格下跌为盈利
                        if price_change >= self.take_profit {
                            // 1. 先取消所有开放订单（包括止损单）
                            match self.aster_api.cancel_all_open_orders(&self.symbol, None).await {
                                Ok(_) => {
                                    order_log!(info, "✅ 止盈操作：成功取消所有开放订单");
                                }
                                Err(e) => {
                                    error_log!(warn, "⚠️ 止盈操作：取消订单失败: {}，继续执行止盈", e);
                                }
                            }
                            
                            // 2. 发出止盈单（市价买入）
                            let take_profit_order = OrderRequest {
                                symbol: self.symbol.clone(),
                                side: OrderSide::Buy,
                                order_type: OrderType::Market,
                                quantity: Some(self.quantity.clone()),
                                reduce_only: Some("true".to_string()),
                                ..Default::default()
                            };
                            
                            match self.aster_api.batch_orders(vec![take_profit_order], None).await {
                                Ok(result) => {
                                    if result.is_all_success() {
                                        println!("✅ 【止盈平仓】空头仓位止盈 - 实盘下单成功");
                                        println!("   开仓价格 (Bid): {:.5}", entry);
                                        println!("   平仓价格 (Bid): {:.5}", current_bid);
                                        println!("   价格变化: {:.5}", price_change);
                                        println!("   盈利: {:.5}", price_change);
                                        println!("   订单ID: {:?}", result.successful_orders.iter().map(|o| o.order_id).collect::<Vec<_>>());
                                        println!("   ────────────────────────────────────────────────────────");
                                        println!();
                                        
                                        order_log!(info, "✅ Lead-Lag 策略止盈成功 - 空头平仓, 盈利: {:.5}", price_change);
                                        
                                        self.current_position = TradeDirection::None;
                                        self.entry_price = None;
                                        self.open_order_ids.clear();
                                    } else {
                                        error_log!(error, "❌ 止盈下单失败 - 部分订单失败");
                                        for (_, error) in &result.failed_orders {
                                            error_log!(error, "   订单失败: code={}, msg={}", error.code, error.msg);
                                        }
                                    }
                                }
                                Err(e) => {
                                    error_log!(error, "❌ 止盈下单失败: {}", e);
                                }
                            }
                        }
                    }
                }
            },
        }
    }

    /// 运行策略主循环
    pub async fn run(&mut self) -> anyhow::Result<()> {
        println!("🚀 Lead-Lag 策略启动");
        println!("📊 监听 Binance 和 ASTER 的 bookTicker 数据");
        println!("📈 策略参数:");
        println!("   入场阈值: {:.5}", self.entry_threshold);
        println!("   止损: {:.5}", self.stop_loss);
        println!("   止盈: {:.5}", self.take_profit);
        println!("   最大允许价差（流动性保护）: {:.5}", self.max_spread);
        println!("{}", "=".repeat(80));


        loop {
            tokio::select! {
                // 处理 Binance bookTicker 数据
                binance_ticker = self.binance_ticker_rx.recv() => {
                    match binance_ticker {
                        Some(ticker) => {
                            // 计算公平价格
                            let fair_price = Self::calculate_fair_price(
                                ticker.best_bid_price,
                                ticker.best_bid_qty,
                                ticker.best_ask_price,
                                ticker.best_ask_qty,
                            );

                            // 更新最新的 Binance fair price
                            self.latest_binance_fair_price = Some(fair_price);

                            // 检查交易机会（开仓需要基于 fair price，但需要订单簿价格才能开仓）
                            self.check_and_execute_trade().await;
                        }
                        None => {
                            println!("⚠️  Binance bookTicker 通道已关闭");
                            break;
                        }
                    }
                }

                // 处理 ASTER bookTicker 数据
                aster_ticker = self.aster_ticker_rx.recv() => {
                    match aster_ticker {
                        Some(ticker) => {
                            // 计算公平价格
                            let fair_price = Self::calculate_fair_price(
                                ticker.best_bid_price,
                                ticker.best_bid_qty,
                                ticker.best_ask_price,
                                ticker.best_ask_qty,
                            );

                            // 更新最新的 ASTER fair price 和订单簿价格
                            self.latest_aster_fair_price = Some(fair_price);
                            self.latest_aster_bid_price = Some(ticker.best_bid_price);
                            self.latest_aster_ask_price = Some(ticker.best_ask_price);

                            // 检查交易机会（开仓和止损止盈都需要检查）
                            self.check_and_execute_trade().await;
                        }
                        None => {
                            println!("⚠️  ASTER bookTicker 通道已关闭");
                            break;
                        }
                    }
                }
            }
        }

        println!("🔚 Lead-Lag 策略结束");
        Ok(())
    }
}

