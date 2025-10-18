pub use crate::models::{CommonDepth, TradeTick, TradeTickBuffer};
pub use tokio::sync::mpsc;

pub struct MEXCOrderBook {
    
}
pub struct SnapShot {
    pub binanceDepth: CommonDepth,
    pub mexcDepth: CommonDepth,
    pub tradeTick: TradeTickBuffer,
}
pub struct SnapshotCreator {
    rec_common_depth: mpsc::Receiver<CommonDepth>,
    rec_trade_tick: mpsc::Receiver<TradeTick>,
    sender_snapshot: mpsc::Sender<SnapShot>,
}

