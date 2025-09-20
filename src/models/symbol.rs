use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{self, Display};
use std::str::FromStr;

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
