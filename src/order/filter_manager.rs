use crate::common::enums::{Exchange, OrderStutus, StrategyName};
use crate::common::signal::TradingSignal;
use std::any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock,mpsc};
use anyhow::Result;
type TokenName=String;

// pub struct OpenPosition {
//     pub position: HashMap<(TokenName,Exchange), f64>,
//     pub strategy_name: StrategyName,
// }
pub struct SignalManager {
    pub open_position: Arc<RwLock<HashMap<StrategyName, f64>>>,
    pub signal_receiver: mpsc::Receiver<TradingSignal>,
}
impl SignalManager{

}
fn macd_filter(signal:TradingSignal,current_position:f64) ->Result<(),anyhow::Error>{

}
// pub struct Order {
//     pub order_id: usize,
//     pub exchange: Exchange,
//     pub symbol: String,
//     pub amount: f64,
//     pub strategy: StrategyName,
//     pub status: OrderStutus,
//     pub timestamp: u64,
//     pub updated_timestamp: u64,
// }

// 首先定义 Filter trait
pub trait SignalFilter: Send + Sync {
    fn filter(&self, signal: &TradingSignal, positions: &HashMap<(Exchange, TokenName), f64>) -> bool;
    fn name(&self) -> String;
}

// 定义一些具体的 Filter
#[derive(Clone)]
pub struct MacdFilter {
    max_position: f64,
    min_position: f64,
}

impl SignalFilter for MacdFilter {
    fn filter(&self, signal: &TradingSignal, positions: &HashMap<(Exchange, TokenName), f64>) -> bool {
        // MACD 策略特定的过滤逻辑
        // 例如：检查持仓限制，检查信号强度等
        true
    }

    fn name(&self) -> String {
        "MACD".to_string()
    }
}

// Filter 管理器
pub struct FilterManager {
    filters: HashMap<StrategyName, Box<dyn SignalFilter>>,
    positions: Arc<RwLock<HashMap<(Exchange, TokenName), f64>>>,
    signal_receiver: mpsc::Receiver<TradingSignal>,
    order_sender: mpsc::Sender<Order>,
}

impl FilterManager {
    pub fn new(
        signal_receiver: mpsc::Receiver<TradingSignal>,
        order_sender: mpsc::Sender<Order>,
        positions: Arc<RwLock<HashMap<(Exchange, TokenName), f64>>>,
    ) -> Self {
        Self {
            filters: HashMap::new(),
            positions,
            signal_receiver,
            order_sender,
        }
    }

    pub fn add_filter(&mut self, strategy: StrategyName, filter: Box<dyn SignalFilter>) {
        self.filters.insert(strategy, filter);
    }

    pub fn process_signals(&mut self) -> Result<()> {
        while let Ok(signal) = self.signal_receiver.recv() {
            // 1. 获取对应策略的 filter
            if let Some(filter) = self.filters.get(&signal.strategy) {
                // 2. 获取当前持仓信息
                let positions = self.positions.read().unwrap();
                
                // 3. 应用过滤规则
                if filter.filter(&signal, &positions) {
                    // 4. 创建订单
                    let order = Order {
                        order_id: self.generate_order_id(),
                        exchange: signal.exchange,
                        symbol: signal.symbol,
                        amount: signal.amount,
                        strategy: signal.strategy,
                        status: OrderStatus::New,
                        timestamp: get_timestamp_ms() as u64,
                        updated_timestamp: get_timestamp_ms() as u64,
                    };

                    // 5. 发送订单
                    self.order_sender.send(order)?;
                }
            }
        }
        Ok(())
    }
}

// 全局风控过滤器（可选）
pub struct GlobalRiskFilter {
    max_total_position: f64,
    max_single_order: f64,
}

impl SignalFilter for GlobalRiskFilter {
    fn filter(&self, signal: &TradingSignal, positions: &HashMap<(Exchange, TokenName), f64>) -> bool {
        // 实现全局风控规则
        true
    }

    fn name(&self) -> String {
        "GlobalRisk".to_string()
    }
}

// 使用示例
pub fn setup_filter_manager() -> Result<FilterManager> {
    let (signal_tx, signal_rx) = mpsc::channel();
    let (order_tx, order_rx) = mpsc::channel();
    let positions = Arc::new(RwLock::new(HashMap::new()));

    let mut filter_manager = FilterManager::new(signal_rx, order_tx, positions);

    // 添加 MACD 策略的过滤器
    filter_manager.add_filter(
        StrategyName::MACD,
        Box::new(MacdFilter {
            max_position: 1.0,
            min_position: -1.0,
        })
    );

    // 可以添加其他策略的过滤器
    // filter_manager.add_filter(StrategyName::RSI, Box::new(RsiFilter::new()));

    Ok(filter_manager)
}
