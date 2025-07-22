use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MEXCDepthTick {
    pub symbol: String,
    pub data: DepthData,
    pub channel: String,
    pub ts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthData {
    pub asks: Vec<[f64; 3]>,
    pub bids: Vec<[f64; 3]>,
    pub end: u64,
    pub begin: u64,
    pub version: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mexc_depth_parsing() {
        let test_str = r#"{
    "symbol": "ME_USDC",
    "data": {
        "asks": [
            [
                0.982,
                40338,
                2
            ],
            [
                0.981,
                7136,
                2
            ]
        ],
        "bids": [
            [
                0.976,
                60670,
                2
            ],
            [
                0.975,
                71988,
                2
            ]
        ],
        "end": 223015984,
        "begin": 223015978,
        "version": 223015984
    },
    "channel": "push.depth",
    "ts": 1753109151378
}"#;
        let data: MEXCDepthTick = serde_json::from_str(test_str).unwrap();
        assert_eq!(data.symbol, "ME_USDC");
        assert_eq!(data.data.asks.len(), 2);
        assert_eq!(data.data.bids.len(), 2);
        assert_eq!(data.data.begin, 223015978);
        assert_eq!(data.data.end, 223015984);
    }
}
