use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display};
use std::str::FromStr;

/// 交易对精度信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SymbolPrecision {
    /// 价格精度（小数位数）
    pub price_precision: u8,
    /// 数量精度（小数位数）
    pub quantity_precision: u8,
}

impl SymbolPrecision {
    pub fn new(price_precision: u8, quantity_precision: u8) -> Self {
        Self {
            price_precision,
            quantity_precision,
        }
    }
}

/// 高效的交易对符号类型
/// 对于常用交易对使用预定义枚举（零成本），对于其他交易对使用固定大小数组
#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum TradingSymbol {
    // 主流交易对 - 零成本枚举
    BTCUSDT,
    ETHUSDT,
    SOLUSDT,
    ADAUSDT,
    XRPUSDT,
    DOGEUSDT,
    TURBOUSDT,
    BNBUSDT,
    AVAXUSDT,
    MATICUSDT,
    DOTUSDT,
    LINKUSDT,
    LTCUSDT,
    UNIUSDT,
    PEPEUSDT,
    NEIROUSDT,
    ONDOUSDT,
    AAVEUSDT,

    // 自定义符号 - 使用固定大小数组 [u8; 20]，完全栈分配，支持 Copy
    Custom([u8; 15]),
}

impl TradingSymbol {
    /// 获取符号的字符串表示
    pub fn as_str(&self) -> &str {
        match self {
            TradingSymbol::BTCUSDT => "BTCUSDT",
            TradingSymbol::ETHUSDT => "ETHUSDT",
            TradingSymbol::SOLUSDT => "SOLUSDT",
            TradingSymbol::ADAUSDT => "ADAUSDT",
            TradingSymbol::XRPUSDT => "XRPUSDT",
            TradingSymbol::DOGEUSDT => "DOGEUSDT",
            TradingSymbol::TURBOUSDT => "TURBOUSDT",
            TradingSymbol::BNBUSDT => "BNBUSDT",
            TradingSymbol::AVAXUSDT => "AVAXUSDT",
            TradingSymbol::MATICUSDT => "MATICUSDT",
            TradingSymbol::DOTUSDT => "DOTUSDT",
            TradingSymbol::LINKUSDT => "LINKUSDT",
            TradingSymbol::LTCUSDT => "LTCUSDT",
            TradingSymbol::UNIUSDT => "UNIUSDT",
            TradingSymbol::PEPEUSDT => "1000PEPEUSDT",
            TradingSymbol::NEIROUSDT => "NEIROUSDT",
            TradingSymbol::ONDOUSDT => "ONDOUSDT",
            TradingSymbol::AAVEUSDT => "AAVEUSDT",
            TradingSymbol::Custom(bytes) => {
                // 找到第一个0字节来确定字符串长度
                let len = bytes.iter().position(|&b| b == 0).unwrap_or(15);
                // 使用 unsafe 来避免额外的分配，因为我们在栈上
                unsafe {
                    std::str::from_utf8_unchecked(&bytes[..len])
                }
            }
        }
    }

    /// 获取交易对的精度信息
    pub fn get_precision(&self) -> SymbolPrecision {
        match self {
            // 主流币种精度配置（基于币安期货）
            TradingSymbol::BTCUSDT => SymbolPrecision::new(2, 3),  // 价格2位，数量3位
            TradingSymbol::ETHUSDT => SymbolPrecision::new(2, 3),  // 价格2位，数量3位
            TradingSymbol::SOLUSDT => SymbolPrecision::new(3, 2),  // 价格3位，数量2位
            TradingSymbol::ADAUSDT => SymbolPrecision::new(4, 0),  // 价格4位，数量0位
            TradingSymbol::XRPUSDT => SymbolPrecision::new(4, 0),  // 价格4位，数量0位
            TradingSymbol::DOGEUSDT => SymbolPrecision::new(5, 0), // 价格5位，数量0位
            TradingSymbol::TURBOUSDT => SymbolPrecision::new(6, 0), // 价格6位，数量0位
            TradingSymbol::BNBUSDT => SymbolPrecision::new(2, 3),  // 价格2位，数量3位
            TradingSymbol::AVAXUSDT => SymbolPrecision::new(3, 2), // 价格3位，数量2位
            TradingSymbol::MATICUSDT => SymbolPrecision::new(4, 0), // 价格4位，数量0位
            TradingSymbol::DOTUSDT => SymbolPrecision::new(3, 2),  // 价格3位，数量2位
            TradingSymbol::LINKUSDT => SymbolPrecision::new(3, 2), // 价格3位，数量2位
            TradingSymbol::LTCUSDT => SymbolPrecision::new(2, 3),  // 价格2位，数量3位
            TradingSymbol::UNIUSDT => SymbolPrecision::new(3, 2),  // 价格3位，数量2位
            TradingSymbol::PEPEUSDT => SymbolPrecision::new(8, 0), // 价格8位，数量0位
            TradingSymbol::NEIROUSDT => SymbolPrecision::new(4, 0), // 价格4位，数量0位
            TradingSymbol::ONDOUSDT => SymbolPrecision::new(4, 0), // 价格4位，数量0位
            TradingSymbol::AAVEUSDT => SymbolPrecision::new(2, 3), // 价格2位，数量3位
            TradingSymbol::Custom(_) => SymbolPrecision::new(6, 3), // 自定义符号默认精度
        }
    }

    /// 根据价格精度调整价格
    pub fn align_price(&self, price: f64) -> f64 {
        let precision = self.get_precision();
        let multiplier = 10_f64.powi(precision.price_precision as i32);
        (price * multiplier).round() / multiplier
    }

    /// 根据数量精度调整数量
    pub fn align_quantity(&self, quantity: f64) -> f64 {
        let precision = self.get_precision();
        let multiplier = 10_f64.powi(precision.quantity_precision as i32);
        (quantity * multiplier).round() / multiplier
    }

    /// 检查是否是预定义的主流交易对
    pub fn is_predefined(&self) -> bool {
        !matches!(self, TradingSymbol::Custom(_))
    }

    /// 从字符串创建符号，自动选择最佳表示方式
    pub fn from_string(s: String) -> Self {
        match s.as_str() {
            "BTCUSDT" => TradingSymbol::BTCUSDT,
            "ETHUSDT" => TradingSymbol::ETHUSDT,
            "SOLUSDT" => TradingSymbol::SOLUSDT,
            "ADAUSDT" => TradingSymbol::ADAUSDT,
            "XRPUSDT" => TradingSymbol::XRPUSDT,
            "DOGEUSDT" => TradingSymbol::DOGEUSDT,
            "TURBOUSDT" => TradingSymbol::TURBOUSDT,
            "BNBUSDT" => TradingSymbol::BNBUSDT,
            "AVAXUSDT" => TradingSymbol::AVAXUSDT,
            "MATICUSDT" => TradingSymbol::MATICUSDT,
            "DOTUSDT" => TradingSymbol::DOTUSDT,
            "LINKUSDT" => TradingSymbol::LINKUSDT,
            "LTCUSDT" => TradingSymbol::LTCUSDT,
            "UNIUSDT" => TradingSymbol::UNIUSDT,
            "1000PEPEUSDT" => TradingSymbol::PEPEUSDT, // 币安实际符号映射
            "NEIROUSDT" => TradingSymbol::NEIROUSDT,
            "ONDOUSDT" => TradingSymbol::ONDOUSDT,
            "AAVEUSDT" => TradingSymbol::AAVEUSDT,
            _ => {
                // 检查字符串长度
                if s.len() > 20 {
                    panic!("Symbol '{}' is too long (max 20 bytes)", s);
                }
                
                // 创建固定大小数组
                let mut bytes = [0u8; 15];
                bytes[..s.len()].copy_from_slice(s.as_bytes());
                TradingSymbol::Custom(bytes)
            }
        }
    }
}

impl FromStr for TradingSymbol {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_string(s.to_string()))
    }
}

impl Display for TradingSymbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<&str> for TradingSymbol {
    fn from(s: &str) -> Self {
        Self::from_string(s.to_string())
    }
}

impl From<String> for TradingSymbol {
    fn from(s: String) -> Self {
        Self::from_string(s)
    }
}

// 为了向后兼容，提供到 String 的转换
impl From<TradingSymbol> for String {
    fn from(symbol: TradingSymbol) -> Self {
        symbol.as_str().to_string()
    }
}

impl Default for TradingSymbol {
    fn default() -> Self {
        TradingSymbol::BTCUSDT // 默认使用最常用的交易对
    }
}

// 自定义序列化：总是序列化为字符串（兼容外部系统）
impl Serialize for TradingSymbol {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

// 自定义反序列化：从字符串反序列化，自动选择最优表示
impl<'de> Deserialize<'de> for TradingSymbol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(TradingSymbol::from_string(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predefined_symbols() {
        let btc = TradingSymbol::BTCUSDT;
        assert_eq!(btc.as_str(), "BTCUSDT");
        assert!(btc.is_predefined());
    }

    #[test]
    fn test_custom_symbols() {
        let custom = TradingSymbol::from_string("NEWCOIN".to_string());
        assert_eq!(custom.as_str(), "NEWCOIN");
        assert!(!custom.is_predefined());
    }

    #[test]
    fn test_from_string() {
        let btc = TradingSymbol::from_string("BTCUSDT".to_string());
        assert_eq!(btc, TradingSymbol::BTCUSDT);

        let custom = TradingSymbol::from_string("UNKNOWNCOIN".to_string());
        let expected = {
            let mut bytes = [0u8; 15];
            bytes[..11].copy_from_slice(b"UNKNOWNCOIN");
            TradingSymbol::Custom(bytes)
        };
        assert_eq!(custom, expected);
    }

    #[test]
    fn test_serialization() {
        let btc = TradingSymbol::BTCUSDT;
        let json = serde_json::to_string(&btc).unwrap();
        assert_eq!(json, r#""BTCUSDT""#); // 序列化为字符串

        let deserialized: TradingSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(btc, deserialized);
        assert!(deserialized.is_predefined()); // 反序列化为预定义枚举
    }

    #[test]
    fn test_custom_symbol_serialization() {
        let custom = TradingSymbol::from_string("NEWCOIN".to_string());
        let json = serde_json::to_string(&custom).unwrap();
        assert_eq!(json, r#""NEWCOIN""#);

        let deserialized: TradingSymbol = serde_json::from_str(&json).unwrap();
        assert_eq!(custom, deserialized);
        assert!(!deserialized.is_predefined());
    }

    #[test]
    fn test_copy_implementation() {
        // 测试预定义符号的 Copy
        let symbol1 = TradingSymbol::BTCUSDT;
        let symbol2 = symbol1;  // 现在可以复制了！
        assert_eq!(symbol1, symbol2);
        
        // 测试自定义符号的 Copy
        let custom1 = TradingSymbol::from_string("NEWCOIN".to_string());
        let custom2 = custom1;  // 也可以复制！
        assert_eq!(custom1, custom2);
        
        // 验证两个变量都可以使用
        assert_eq!(symbol1.as_str(), "BTCUSDT");
        assert_eq!(symbol2.as_str(), "BTCUSDT");
        assert_eq!(custom1.as_str(), "NEWCOIN");
        assert_eq!(custom2.as_str(), "NEWCOIN");
    }

    #[test]
    fn test_fixed_array_symbol() {
        // 测试固定大小数组的符号
        let symbol = TradingSymbol::from_string("BTCUSDT".to_string());
        assert_eq!(symbol.as_str(), "BTCUSDT");
        
        // 测试长符号
        let long_symbol = TradingSymbol::from_string("1000PEPEUSDT".to_string());
        assert_eq!(long_symbol.as_str(), "1000PEPEUSDT");
        
        // 测试边界情况 - 正好20字节
        let max_symbol = TradingSymbol::from_string("123456789012345".to_string());
        assert_eq!(max_symbol.as_str(), "123456789012345");
        
        // 测试复制
        let symbol_copy = symbol;
        assert_eq!(symbol, symbol_copy);
        assert_eq!(symbol.as_str(), symbol_copy.as_str());
    }

    #[test]
    #[should_panic(expected = "Symbol 'VERY_LONG_SYMBOL_NAME_THAT_EXCEEDS_TWENTY_BYTES' is too long")]
    fn test_symbol_too_long() {
        // 测试超过20字节的符号应该panic
        TradingSymbol::from_string("VERY_LONG_SYMBOL_NAME_THAT_EXCEEDS_TWENTY_BYTES".to_string());
    }

    #[test]
    fn test_symbol_precision() {
        // 测试BTC精度
        let btc = TradingSymbol::BTCUSDT;
        let btc_precision = btc.get_precision();
        assert_eq!(btc_precision.price_precision, 2);
        assert_eq!(btc_precision.quantity_precision, 3);
        
        // 测试PEPE精度
        let pepe = TradingSymbol::PEPEUSDT;
        let pepe_precision = pepe.get_precision();
        assert_eq!(pepe_precision.price_precision, 8);
        assert_eq!(pepe_precision.quantity_precision, 0);
        
        // 测试ETH精度
        let eth = TradingSymbol::ETHUSDT;
        let eth_precision = eth.get_precision();
        assert_eq!(eth_precision.price_precision, 2);
        assert_eq!(eth_precision.quantity_precision, 3);
    }

    #[test]
    fn test_price_alignment() {
        let btc = TradingSymbol::BTCUSDT;
        
        // 测试BTC价格对齐（2位小数）
        assert_eq!(btc.align_price(65000.123456), 65000.12);
        assert_eq!(btc.align_price(65000.999), 65001.0);
        assert_eq!(btc.align_price(65000.0), 65000.0);
        
        let pepe = TradingSymbol::PEPEUSDT;
        
        // 测试PEPE价格对齐（8位小数）
        assert_eq!(pepe.align_price(0.0000123456789), 0.00001235);
        assert_eq!(pepe.align_price(0.0000123456781), 0.00001235);
        
        let eth = TradingSymbol::ETHUSDT;
        
        // 测试ETH价格对齐（2位小数）
        assert_eq!(eth.align_price(3200.567), 3200.57);
        assert_eq!(eth.align_price(3200.123), 3200.12);
    }

    #[test]
    fn test_quantity_alignment() {
        let btc = TradingSymbol::BTCUSDT;
        
        // 测试BTC数量对齐（3位小数）
        assert_eq!(btc.align_quantity(0.0012345), 0.001);
        assert_eq!(btc.align_quantity(0.0015678), 0.002);
        assert_eq!(btc.align_quantity(1.0), 1.0);
        
        let pepe = TradingSymbol::PEPEUSDT;
        
        // 测试PEPE数量对齐（0位小数，整数）
        assert_eq!(pepe.align_quantity(1000.5), 1001.0);
        assert_eq!(pepe.align_quantity(1000.4), 1000.0);
        assert_eq!(pepe.align_quantity(1000.0), 1000.0);
        
        let eth = TradingSymbol::ETHUSDT;
        
        // 测试ETH数量对齐（3位小数）
        assert_eq!(eth.align_quantity(0.123456), 0.123);
        assert_eq!(eth.align_quantity(0.123567), 0.124);
    }

    #[test]
    fn test_custom_symbol_precision() {
        let custom = TradingSymbol::from_string("NEWCOIN".to_string());
        let precision = custom.get_precision();
        
        // 自定义符号使用默认精度
        assert_eq!(precision.price_precision, 6);
        assert_eq!(precision.quantity_precision, 3);
        
        // 测试自定义符号的价格和数量对齐
        assert_eq!(custom.align_price(123.456789), 123.456789);
        assert_eq!(custom.align_quantity(0.123456), 0.123);
    }

    #[test]
    fn test_performance_comparison() {
        // 创建测试数据 - 混合预定义和自定义符号
        let enum_symbols = vec![
            TradingSymbol::BTCUSDT,
            TradingSymbol::ETHUSDT,
            TradingSymbol::SOLUSDT,
            TradingSymbol::from_string("NEWCOIN1".to_string()),
            TradingSymbol::from_string("NEWCOIN2".to_string()),
        ];

        let string_symbols = vec![
            "BTCUSDT".to_string(),
            "ETHUSDT".to_string(),
            "SOLUSDT".to_string(),
            "NEWCOIN1".to_string(),
            "NEWCOIN2".to_string(),
        ];

        let target_enum = TradingSymbol::ETHUSDT;
        let target_string = "ETHUSDT".to_string();

        // 测试枚举比较 - 在集合中查找
        let start = std::time::Instant::now();
        for _ in 0..100000 {
            for symbol in &enum_symbols {
                let _ = *symbol == target_enum;
            }
        }
        let enum_time = start.elapsed();

        // 测试字符串比较 - 在集合中查找
        let start = std::time::Instant::now();
        for _ in 0..100000 {
            for symbol in &string_symbols {
                let _ = *symbol == target_string;
            }
        }
        let str_time = start.elapsed();

        println!("Enum comparison (100k * 5 items): {:?}", enum_time);
        println!("String comparison (100k * 5 items): {:?}", str_time);

        // 测试哈希性能 - 这是enum真正的优势
        use std::collections::HashSet;

        let mut enum_set = HashSet::new();
        let mut string_set = HashSet::new();

        // 插入性能测试
        let start = std::time::Instant::now();
        for _ in 0..50000 {
            for symbol in &enum_symbols {
                enum_set.insert(*symbol);  // 现在可以使用 Copy 而不是 clone
            }
        }
        let enum_hash_time = start.elapsed();

        let start = std::time::Instant::now();
        for _ in 0..50000 {
            for symbol in &string_symbols {
                string_set.insert(symbol.clone());
            }
        }
        let str_hash_time = start.elapsed();

        println!("Enum hash insert (50k * 5 items): {:?}", enum_hash_time);
        println!("String hash insert (50k * 5 items): {:?}", str_hash_time);

        // 查找性能测试
        let start = std::time::Instant::now();
        for _ in 0..100000 {
            let _ = enum_set.contains(&target_enum);
        }
        let enum_lookup_time = start.elapsed();

        let start = std::time::Instant::now();
        for _ in 0..100000 {
            let _ = string_set.contains(&target_string);
        }
        let str_lookup_time = start.elapsed();

        println!("Enum hash lookup (100k): {:?}", enum_lookup_time);
        println!("String hash lookup (100k): {:?}", str_lookup_time);
    }
}
