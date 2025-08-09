use crate::common::enums::{Exchange, OrderStutus, StrategyName};
use crate::common::signal::{TradingSignal, Side};
use crate::exchange_api::binance::api::BinanceFuturesApi;
use crate::dto::binance::rest_api::{OrderRequest, OrderSide, OrderType};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use anyhow::Result;

pub struct SignalManager {
    pub open_position: Arc<RwLock<HashMap<StrategyName, f64>>>,
    pub signal_receiver: mpsc::Receiver<TradingSignal>,
    binance_client: BinanceFuturesApi,
}

impl SignalManager {
    pub fn new(
        signal_receiver: mpsc::Receiver<TradingSignal>,
        open_position: Arc<RwLock<HashMap<StrategyName, f64>>>,
        api_key: String,
        secret_key: String,
    ) -> Self {
        let binance_client = BinanceFuturesApi::new(api_key, secret_key);
        Self {
            open_position,
            signal_receiver,
            binance_client,
        }
    }

    pub async fn process_signals(&mut self) -> Result<()> {
        // 使用多个任务并发处理信号
        while let Some(signal) = self.signal_receiver.recv().await {
            // 克隆需要的数据用于新任务
            let open_position = self.open_position.clone();
            let client = self.binance_client.clone();
            
            // 启动新的任务处理信号
            tokio::spawn(async move {
                if let Err(e) = Self::process_single_signal(signal, open_position, client).await {
                    eprintln!("Signal processing error: {}", e);
                }
            });
        }
        Ok(())
    }

    async fn process_single_signal(
        signal: TradingSignal,
        open_position: Arc<RwLock<HashMap<StrategyName, f64>>>,
        client: BinanceFuturesApi,
    ) -> Result<()> {
        let strategy = signal.strategy;

        // 1. 先检查并更新仓位（乐观锁模式）
        {
            let mut positions = open_position.write().await;
            let current_position = positions.get(&strategy).copied().unwrap_or(0.0);
            
            // 如果已有仓位，拒绝信号
            if current_position != 0.0 {
                println!(
                    "Signal rejected: Current position: {}, Symbol: {}", 
                    current_position, 
                    signal.symbol
                );
                return Ok(());
            }

            // 乐观地更新仓位
            positions.insert(strategy, signal.quantity);
        }

        // 2. 准备下单请求
        let order_side = match signal.side {
            Side::Buy => OrderSide::Buy,
            Side::Sell => OrderSide::Sell,
        };

        let order_request = OrderRequest {
            symbol: signal.symbol.clone(),
            side: order_side,
            order_type: OrderType::Market,
            quantity: Some(signal.quantity.to_string()),
            timestamp: Some(BinanceFuturesApi::get_timestamp()),
            recv_window: Some(60000),
            ..Default::default()
        };

        // 3. 发送订单（异步）
        match client.new_order(order_request).await {
            Ok(response) => {
                println!(
                    "Order placed successfully: Symbol: {}, Side: {:?}, Quantity: {}, OrderId: {}", 
                    signal.symbol, 
                    signal.side,
                    signal.quantity,
                    response.order_id
                );
                Ok(())
            }
            Err(e) => {
                // 下单失败，回滚仓位
                let mut positions = open_position.write().await;
                positions.remove(&strategy);
                
                eprintln!("Failed to place order: {}", e);
                Err(anyhow::anyhow!("Failed to place order: {}", e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::signal::{Side, Signal, MarketSignal};
    
    #[tokio::test]
    async fn test_concurrent_signal_processing() {
        let api_key = std::env::var("BINANCE_API_KEY").expect("Missing BINANCE_API_KEY");
        let secret_key = std::env::var("BINANCE_SECRET_KEY").expect("Missing BINANCE_SECRET_KEY");
        
        let (signal_tx, signal_rx) = mpsc::channel(100);
        let positions = Arc::new(RwLock::new(HashMap::new()));
        
        let mut manager = SignalManager::new(
            signal_rx, 
            positions.clone(),
            api_key,
            secret_key,
        );

        // 创建多个测试信号
        let test_signals = vec![
            TradingSignal::new_market_signal(
                1, "BTCUSDT".to_string(), Side::Buy, StrategyName::MACD,
                0.001, Exchange::Binance, 0, None, None, 50000.0,
            ),
            TradingSignal::new_market_signal(
                2, "ETHUSDT".to_string(), Side::Buy, StrategyName::HBFC,
                0.01, Exchange::Binance, 0, None, None, 3000.0,
            ),
        ];

        // 并发发送信号
        for signal in test_signals {
            let signal_tx = signal_tx.clone();
            tokio::spawn(async move {
                signal_tx.send(signal).await.unwrap();
            });
        }

        // 运行信号处理
        manager.process_signals().await.unwrap();
    }
}
