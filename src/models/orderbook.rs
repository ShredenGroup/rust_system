use crate::dto::mexc::PushDataV3ApiWrapper;
use crate::models::{Exchange, TradingSymbol};
use std::collections::BTreeMap;
use crate::common::utils::f2u;
type Price = u64;
type Quantity = u64;
#[derive(Debug, Clone)]
pub struct CommonDepth {
    pub bid_list: BTreeMap<Price, Quantity>,
    pub ask_list: BTreeMap<Price, Quantity>,
    pub symbol: TradingSymbol,
    pub timestamp: i64,
    pub exchange: Exchange,
}

// 可以在这里添加其他 MEXC 相关的结构体
impl CommonDepth {
    pub fn new_from_mexc(data: PushDataV3ApiWrapper) -> Option<Self> {
        if let Some(partial_depth) = data.extract_limit_depth_data() {
            // 辅助函数：将深度数据转换为 BTreeMap
            let depth_to_map = |items: &[crate::dto::mexc::PublicLimitDepthV3ApiItem]| {
                items
                    .iter()
                    .filter_map(|item| {
                        item.price.parse::<f64>()
                            .ok()
                            .and_then(|price| {
                                item.quantity.parse::<f64>()
                                    .ok()
                                    .map(|quantity| (f2u(price), f2u(quantity)))
                            })
                    })
                    .collect::<BTreeMap<Price, Quantity>>()
            };
            
            Some(CommonDepth {
                bid_list: depth_to_map(&partial_depth.bids),
                ask_list: depth_to_map(&partial_depth.asks),
                symbol: data.symbol
                    .map(TradingSymbol::from)
                    .unwrap_or(TradingSymbol::BTCUSDT),
                timestamp: data.create_time.unwrap_or_else(|| data.send_time.unwrap_or(0)),
                exchange: Exchange::Mexc,
            })
        } else {
            None
        }
    }
}
