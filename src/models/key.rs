use super::{Exchange, StrategyType, TradingSymbol};
#[derive(Copy, Clone, Debug)]
pub struct CommonKey {
    pub exchange: Exchange,
    pub strategy: StrategyType,
    pub symbol: TradingSymbol,
}
