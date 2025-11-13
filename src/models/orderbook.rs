use crate::dto::binance::websocket::BinancePartialDepth;
use crate::dto::mexc::PushDataV3ApiWrapper;
use crate::models::{Exchange, TradingSymbol};
use std::collections::BTreeMap;
use ordered_float::OrderedFloat;

type Price = OrderedFloat<f64>;
type Quantity = f64;
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
                        item.price.parse::<f64>().ok().and_then(|price| {
                            item.quantity
                                .parse::<f64>()
                                .ok()
                                .map(|quantity| (OrderedFloat(price), quantity))
                        })
                    })
                    .collect::<BTreeMap<Price, Quantity>>()
            };

            Some(CommonDepth {
                bid_list: depth_to_map(&partial_depth.bids),
                ask_list: depth_to_map(&partial_depth.asks),
                symbol: data
                    .symbol
                    .map(TradingSymbol::from)
                    .unwrap_or(TradingSymbol::BTCUSDT),
                timestamp: data
                    .create_time
                    .unwrap_or_else(|| data.send_time.unwrap_or(0)),
                exchange: Exchange::Mexc,
            })
        } else {
            None
        }
    }

    pub fn new_from_binance(data: BinancePartialDepth) -> Self {
        // 辅助函数：将 Binance 深度数据转换为 BTreeMap
        let depth_to_map = |items: &[[f64; 2]]| {
            items
                .iter()
                .filter_map(|item| {
                    let price = item[0];
                    let quantity = item[1];
                    // 过滤掉价格为0或数量为0的无效数据
                    if price > 0.0 && quantity > 0.0 {
                        Some((OrderedFloat(price), quantity))
                    } else {
                        None
                    }
                })
                .collect::<BTreeMap<Price, Quantity>>()
        };

        CommonDepth {
            bid_list: depth_to_map(&data.bids),
            ask_list: depth_to_map(&data.asks),
            symbol: TradingSymbol::BTCUSDT, // Partial Depth 没有 symbol，使用默认值
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64,
            exchange: Exchange::Binance,
        }
    }
}
// 注意：由于我们使用 f64 作为价格类型，不再实现 ta::Orderbook trait
// 该 trait 期望 BTreeMap<u64, u64>，但我们使用 OrderedFloat<f64> 作为 key
// 如果需要使用 ta 库的功能，可以通过访问 bid_list 和 ask_list 字段来获取数据