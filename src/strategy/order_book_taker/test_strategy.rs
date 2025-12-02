use ta::order_ticker_indicators::OtQuantityF64;
use ta::trade_ticker_indicator::TakerBuyRatioF64;
use ta::ob_indicators::VolumeImbalanceF64;
use ta::Next; // 需要导入 Next trait 才能使用 next() 方法
use crate::common::ts::OrderBookStrategy;
use crate::models::{strategy::StrategyContext, TradingSignal};
use crate::middle_processor::snapshot_creator::SnapShot;

pub struct TestStrategy {
    pub cxt: StrategyContext,
    pub order_tick_quantity: OtQuantityF64,
    pub volume_imbalance: VolumeImbalanceF64,
    pub taker_buy_ratio: TakerBuyRatioF64,
}

impl OrderBookStrategy<&SnapShot> for TestStrategy {
    fn on_orderbook_update(&mut self, input: &SnapShot) -> Option<TradingSignal> {
        // 更新技术指标并获取值
        let order_tick_qty = self.order_tick_quantity.extract(input);
        let volume_imb = self.volume_imbalance.next(&input.binance_depth);
        let taker_buy_ratio = self.taker_buy_ratio.next(input);

        // 打印技术指标值
        println!("📊 技术指标:");
        println!("  - OrderTick Quantity: {}", order_tick_qty);
        println!("  - Volume Imbalance: {:.6}", volume_imb);
        println!("  - Taker Buy Ratio: {:.6}", taker_buy_ratio);
        None
    }

    fn strategy_cxt(&self) -> StrategyContext {
        self.cxt.clone()
    }
}
