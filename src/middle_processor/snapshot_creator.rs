pub use crate::dto::mexc::PushDataV3ApiWrapper;
pub use crate::dto::binance::websocket::{BinancePartialDepth, BookTickerData,BinanceTradeData};
pub use crate::models::{CommonDepth, OrderTick, OrderTickBuffer, TradeTick, TradeTickBuffer};
pub use tokio::sync::mpsc;
pub use ta::{TradeTickerf64,OrderTickerf64,BatchTradeTickerf64,BatchOrderTickerf64,Orderbookf64};
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;
pub struct SnapShot {
    pub binance_depth: CommonDepth,
    pub mexc_order_tick: OrderTick,
    pub order_tick: OrderTickBuffer,
    pub trade_tick: TradeTickBuffer,
}

pub struct SnapshotCreator {
    pub rec_mexc_order_tick: mpsc::Receiver<PushDataV3ApiWrapper>,
    pub rec_binance_depth: mpsc::Receiver<BinancePartialDepth>,
    pub rec_order_tick: mpsc::Receiver<BookTickerData>,
    pub rec_trade_tick: mpsc::Receiver<BinanceTradeData>,
    pub sender_snapshot: mpsc::Sender<SnapShot>,
}

impl SnapshotCreator {
    pub fn new(rec_mexc_order_tick: mpsc::Receiver<PushDataV3ApiWrapper>,
    rec_binance_depth: mpsc::Receiver<BinancePartialDepth>,
    rec_order_tick: mpsc::Receiver<BookTickerData>,
    rec_trade_tick: mpsc::Receiver<BinanceTradeData>,
    sender_snapshot: mpsc::Sender<SnapShot>) -> Self {
        Self {
            rec_mexc_order_tick,
            rec_binance_depth,
            rec_order_tick,
            rec_trade_tick,
            sender_snapshot,
        }
    }

    /// 启动快照创建器的主循环
    /// 
    /// 处理逻辑：
    /// 1. TradeTick 数据持续存储到 TradeTickBuffer 中
    /// 2. OrderTick 数据持续存储到 OrderTickBuffer 中
    /// 3. MEXC OrderTick 数据持续更新
    /// 4. 当 BinanceDepth 数据到达时，触发快照创建并发送
    /// 5. 如果某些数据没有更新，使用旧数据
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut trade_buffer = TradeTickBuffer::new(1000); // 最多存储1000笔交易
        let mut order_buffer = OrderTickBuffer::new(1000); // 最多存储1000个订单tick
        let mut latest_mexc_tick: Option<OrderTick> = None;
        
        println!("🚀 SnapshotCreator 启动，开始处理数据流...");

        loop {
            tokio::select! {
                // 处理 TradeTick 数据
                trade_tick = self.rec_trade_tick.recv() => {
                    match trade_tick {
                        Some(trade_data) => {
                            // 将 BinanceTradeData 转换为 TradeTick 并存储到缓冲区
                            let tick = TradeTick::new_from_binance(trade_data);
                            trade_buffer.push_trade(tick);
                            println!("📊 收到 TradeTick，当前缓冲区大小: {}", trade_buffer.len());
                        }
                        None => {
                            println!("⚠️ TradeTick 通道已关闭");
                            break;
                        }
                    }
                }

                // 处理 OrderTick 数据
                order_tick = self.rec_order_tick.recv() => {
                    match order_tick {
                        Some(order_data) => {
                            // 将 BookTickerData 转换为 OrderTick 并存储到缓冲区
                            let tick = OrderTick::new_from_binance(order_data);
                            order_buffer.push_tick(tick);
                            println!("📈 收到 OrderTick，当前缓冲区大小: {}", order_buffer.len());
                        }
                        None => {
                            println!("⚠️ OrderTick 通道已关闭");
                            break;
                        }
                    }
                }

                // 处理 MEXC OrderTick 数据
                mexc_data = self.rec_mexc_order_tick.recv() => {
                    match mexc_data {
                        Some(data) => {
                            // 尝试从 MEXC 数据中提取 OrderTick
                            match OrderTick::new_from_mexc(data) {
                                Ok(order_tick) => {
                                    latest_mexc_tick = Some(order_tick);
                                    println!("📈 更新 MEXC OrderTick: bid={}, ask={}", 
                                        latest_mexc_tick.as_ref().unwrap().data.best_bid_price,
                                        latest_mexc_tick.as_ref().unwrap().data.best_ask_price);
                                }
                                Err(e) => {
                                    println!("❌ 解析 MEXC OrderTick 失败: {}", e);
                                }
                            }
                        }
                        None => {
                            println!("⚠️ MEXC OrderTick 通道已关闭");
                            break;
                        }
                    }
                }

                // 处理 BinanceDepth 数据 - 这是触发快照的关键
                binance_depth = self.rec_binance_depth.recv() => {
                    match binance_depth {
                        Some(depth_data) => {
                            println!("🎯 收到 BinanceDepth，准备创建快照...");
                            
                            // 将 BinanceDepth 转换为 CommonDepth
                            let common_depth = CommonDepth::new_from_binance(depth_data);
                            
                            // 获取最新的 MEXC OrderTick，如果没有则使用默认值
                            let mexc_tick = latest_mexc_tick.clone().unwrap_or_else(|| {
                                println!("⚠️ 没有最新的 MEXC OrderTick，使用默认值");
                                OrderTick {
                                    data: crate::models::order_tick::OrderTickData {
                                        best_bid_price: 0.0,
                                        best_ask_price: 0.0,
                                        best_bid_quantity: 0.0,
                                        best_ask_quantity: 0.0,
                                    },
                                    exchange: crate::models::Exchange::Mexc,
                                    symbol: crate::models::TradingSymbol::BTCUSDT,
                                    timestamp: 0,
                                }
                            });
                            
                            // 创建快照，直接移动缓冲区所有权（不克隆）
                            let snapshot = SnapShot {
                                binance_depth: common_depth,
                                mexc_order_tick: mexc_tick,
                                order_tick: order_buffer,  // 直接移动所有权
                                trade_tick: trade_buffer,  // 直接移动所有权
                            };
                            
                            // 发送前打印详细信息
                            println!("📊 准备发送快照: Binance深度={}档, MEXC tick={}, OrderTick数={}, 交易数={}", 
                                snapshot.binance_depth.bid_list.len() + snapshot.binance_depth.ask_list.len(),
                                latest_mexc_tick.is_some(),
                                snapshot.order_tick.len(),
                                snapshot.trade_tick.len());
                            
                            // 发送快照
                            match self.sender_snapshot.send(snapshot).await {
                                Ok(_) => {
                                    println!("✅ 快照发送成功");
                                    // 发送后重新创建新的缓冲区来继续接收数据
                                    trade_buffer = TradeTickBuffer::new(1000);
                                    order_buffer = OrderTickBuffer::new(1000);
                                }
                                Err(e) => {
                                    println!("❌ 快照发送失败: {}", e);
                                    break;
                                }
                            }
                        }
                        None => {
                            println!("⚠️ BinanceDepth 通道已关闭");
                            break;
                        }
                    }
                }
            }
        }

        println!("🛑 SnapshotCreator 主循环结束");
        Ok(())
    }
}
impl BatchOrderTickerf64<OrderTick> for SnapShot{
    fn get_batch_order_ticker(&self) -> Option<&[OrderTick]> {
        self.order_tick.get_batch_order_ticker()
    }
}
impl BatchTradeTickerf64<TradeTick> for SnapShot{
    fn get_batch_trade_ticker(&self) -> Option<&[TradeTick]> {
        self.trade_tick.get_batch_trade_ticker()
    }
}
impl Orderbookf64 for SnapShot{
    fn get_bids_btm(&self) -> &BTreeMap<OrderedFloat<f64>, f64> {
        &self.binance_depth.bid_list
    }
    fn get_asks_btm(&self) -> &BTreeMap<OrderedFloat<f64>, f64> {
        &self.binance_depth.ask_list
    }
}