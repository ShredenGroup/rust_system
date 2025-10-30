use crate::dto::binance::websocket::BookTickerData as BinanceBookTickerData;
use crate::dto::aster::websocket::AsterBookTickerData;
use tokio::sync::mpsc;

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
    
    // 最新的 fair price（用于开仓判断）
    latest_binance_fair_price: Option<f64>,
    latest_aster_fair_price: Option<f64>,
    
    // 最新的 ASTER 订单簿价格（用于止损止盈判断）
    latest_aster_bid_price: Option<f64>,
    latest_aster_ask_price: Option<f64>,
    
    // 当前持仓状态
    current_position: TradeDirection,
    entry_price: Option<f64>, // 开仓价格（使用订单簿价格：做多用ask，做空用bid）
    
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
    ) -> Self {
        Self {
            binance_ticker_rx,
            aster_ticker_rx,
            latest_binance_fair_price: None,
            latest_aster_fair_price: None,
            latest_aster_bid_price: None,
            latest_aster_ask_price: None,
            current_position: TradeDirection::None,
            entry_price: None,
            entry_threshold: 0.0003,
            stop_loss: 0.0001,
            take_profit: 0.0003,
            max_spread: 0.0001,
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
    fn check_and_execute_trade(&mut self) {
        let binance_price = match self.latest_binance_fair_price {
            Some(p) => p,
            None => return,
        };
        
        let aster_price = match self.latest_aster_fair_price {
            Some(p) => p,
            None => return,
        };

        // 计算价差
        let price_diff = binance_price - aster_price;

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
                
                // Binance fair price > ASTER fair price + 0.0003 -> 在 ASTER 做多（用 ask 价格开仓）
                if price_diff > self.entry_threshold {
                    self.current_position = TradeDirection::Long;
                    self.entry_price = Some(aster_ask); // 做多用 ask 价格
                    println!("🟢 【开仓】在 ASTER 做多");
                    println!("   开仓价格 (Ask): {:.5}", aster_ask);
                    println!("   Binance Fair Price: {:.5}", binance_price);
                    println!("   ASTER Fair Price: {:.5}", aster_price);
                    println!("   ASTER 价差: {:.5} (Bid: {:.5}, Ask: {:.5})", aster_spread, aster_bid, aster_ask);
                    println!("   价差: {:.5} (超过阈值 {:.5})", price_diff, self.entry_threshold);
                    println!("   止损价格: {:.5} (Ask价格下跌 {:.5})", aster_ask - self.stop_loss, self.stop_loss);
                    println!("   止盈价格: {:.5} (Ask价格上涨 {:.5})", aster_ask + self.take_profit, self.take_profit);
                    println!("   ────────────────────────────────────────────────────────");
                    println!();
                }
                // ASTER fair price > Binance fair price + 0.0003 -> 在 ASTER 做空（用 bid 价格开仓）
                else if -price_diff > self.entry_threshold {
                    self.current_position = TradeDirection::Short;
                    self.entry_price = Some(aster_bid); // 做空用 bid 价格
                    println!("🔴 【开仓】在 ASTER 做空");
                    println!("   开仓价格 (Bid): {:.5}", aster_bid);
                    println!("   Binance Fair Price: {:.5}", binance_price);
                    println!("   ASTER Fair Price: {:.5}", aster_price);
                    println!("   ASTER 价差: {:.5} (Bid: {:.5}, Ask: {:.5})", aster_spread, aster_bid, aster_ask);
                    println!("   价差: {:.5} (超过阈值 {:.5})", -price_diff, self.entry_threshold);
                    println!("   止损价格: {:.5} (Bid价格上涨 {:.5})", aster_bid + self.stop_loss, self.stop_loss);
                    println!("   止盈价格: {:.5} (Bid价格下跌 {:.5})", aster_bid - self.take_profit, self.take_profit);
                    println!("   ────────────────────────────────────────────────────────");
                    println!();
                }
            }
            
            TradeDirection::Long => {
                // 持有多头仓位，检查止损和止盈
                // 做多时，使用 ASTER 的 ask 价格来判断（买入时用卖价）
                let current_ask = match self.latest_aster_ask_price {
                    Some(p) => p,
                    None => return, // 没有订单簿数据，无法判断
                };
                
                if let Some(entry) = self.entry_price {
                    // 做多：当前 ask 价格相比开仓时的 ask 价格
                    let price_change = current_ask - entry;
                    
                    // 止损：ask 价格下跌超过 0.0001
                    if price_change <= -self.stop_loss {
                        println!("⛔ 【止损平仓】多头仓位止损");
                        println!("   开仓价格 (Ask): {:.5}", entry);
                        println!("   平仓价格 (Ask): {:.5}", current_ask);
                        println!("   价格变化: {:.5}", price_change);
                        println!("   亏损: {:.5}", price_change);
                        println!("   ────────────────────────────────────────────────────────");
                        println!();
                        self.current_position = TradeDirection::None;
                        self.entry_price = None;
                    }
                    // 止盈：ask 价格上涨超过 0.0003
                    else if price_change >= self.take_profit {
                        println!("✅ 【止盈平仓】多头仓位止盈");
                        println!("   开仓价格 (Ask): {:.5}", entry);
                        println!("   平仓价格 (Ask): {:.5}", current_ask);
                        println!("   价格变化: {:.5}", price_change);
                        println!("   盈利: {:.5}", price_change);
                        println!("   ────────────────────────────────────────────────────────");
                        println!();
                        self.current_position = TradeDirection::None;
                        self.entry_price = None;
                    }
                }
            }
            
            TradeDirection::Short => {
                // 持有空头仓位，检查止损和止盈
                // 做空时，使用 ASTER 的 bid 价格来判断（卖出时用买价）
                let current_bid = match self.latest_aster_bid_price {
                    Some(p) => p,
                    None => return, // 没有订单簿数据，无法判断
                };
                
                if let Some(entry) = self.entry_price {
                    // 做空：当前 bid 价格相比开仓时的 bid 价格
                    let price_change = entry - current_bid; // 做空：价格下跌为盈利
                    
                    // 止损：bid 价格上涨超过 0.0001（对空头不利）
                    if price_change <= -self.stop_loss {
                        println!("⛔ 【止损平仓】空头仓位止损");
                        println!("   开仓价格 (Bid): {:.5}", entry);
                        println!("   平仓价格 (Bid): {:.5}", current_bid);
                        println!("   价格变化: {:.5}", price_change);
                        println!("   亏损: {:.5}", -price_change);
                        println!("   ────────────────────────────────────────────────────────");
                        println!();
                        self.current_position = TradeDirection::None;
                        self.entry_price = None;
                    }
                    // 止盈：bid 价格下跌超过 0.0003（对空头有利）
                    else if price_change >= self.take_profit {
                        println!("✅ 【止盈平仓】空头仓位止盈");
                        println!("   开仓价格 (Bid): {:.5}", entry);
                        println!("   平仓价格 (Bid): {:.5}", current_bid);
                        println!("   价格变化: {:.5}", price_change);
                        println!("   盈利: {:.5}", price_change);
                        println!("   ────────────────────────────────────────────────────────");
                        println!();
                        self.current_position = TradeDirection::None;
                        self.entry_price = None;
                    }
                }
            }
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
                            self.check_and_execute_trade();
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
                            self.check_and_execute_trade();
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

