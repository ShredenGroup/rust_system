use crate::common::consts::PARSE_DECIMAL;
use hmac::{Hmac, Mac};
use std::num::ParseIntError;
use sha2::Sha256;
use std::time::{SystemTime, UNIX_EPOCH};
use std::num::ParseFloatError;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
/// # HMAC-SHA256 签名生成器
///
/// 使用 HMAC-SHA256 算法为给定的查询字符串生成一个签名。
///
/// ## 参数
/// - `query_string`: 需要被签名的URL编码的查询字符串。
/// - `secret_key`: 用于签名的 API Secret Key。
///
/// ## 返回
///
/// 返回一个十六进制编码的签名字符串。
///
pub fn generate_hmac_signature(query_string: &str, secret_key: &str) -> String {
    type HmacSha256 = Hmac<Sha256>;

    let mut mac =
        HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can take a key of any size");

    mac.update(query_string.as_bytes());

    let result = mac.finalize();
    let code_bytes = result.into_bytes();

    hex::encode(code_bytes)
}

pub fn get_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

/// 将目标价格与参考价格对齐精度
///
/// # 参数
/// - `reference_price`: 参考价格（通常是市场价格）
/// - `target_price`: 需要调整精度的目标价格（通常是止损价格）
///
/// # 返回
/// 返回与参考价格精度对齐的目标价格
///
/// # 示例
/// ```
/// use crate::common::utils::align_price_precision;
///
/// let market_price = 95000.12;  // 市场价格，2位小数
/// let stop_price = 94500.123456; // 止损价格，6位小数
/// let aligned_stop_price = align_price_precision(market_price, stop_price);
/// // 结果: 94500.12 (与市场价格保持相同的2位小数精度)
/// ```
pub fn align_price_precision(reference_price: f64, target_price: f64) -> f64 {
    // 使用更可靠的方法：通过分析参考价格的字符串表示来确定精度
    // 使用固定格式来确保小数位数的一致性
    let reference_str = format!("{:.10}", reference_price);

    // 找到小数点的位置
    if let Some(dot_pos) = reference_str.find('.') {
        // 找到有效数字的结束位置（去除尾随的0）
        let mut end_pos = reference_str.len();
        for (i, ch) in reference_str.chars().rev().enumerate() {
            if ch != '0' {
                end_pos = reference_str.len() - i;
                break;
            }
        }

        // 计算小数位数
        let decimal_places = end_pos - dot_pos - 1;

        // 根据小数位数调整目标价格
        let multiplier = 10_f64.powi(decimal_places as i32);
        (target_price * multiplier).round() / multiplier
    } else {
        // 如果没有小数点，说明是整数，返回整数
        target_price.round()
    }
}

#[inline]
pub fn f2u(data: f64) -> u64 {
    // 使用 round() 确保四舍五入，然后转换为 u64
    (data * PARSE_DECIMAL).round() as u64
}
#[inline]
pub fn s2u_custom(input: &str) -> Result<u64, ParseFloatError> {
    // 高性能版本：避免 Vec 分配，直接解析
    if let Some(dot_pos) = input.find('.') {
        // 有小数点的情况
        let integer_part = &input[..dot_pos];
        let decimal_part = &input[dot_pos + 1..];
        
        let integer = integer_part.parse::<u64>().unwrap_or(0);
        let decimal_len = decimal_part.len();
        
        if decimal_len == 0 {
            Ok(integer * PARSE_DECIMAL as u64)
        } else if decimal_len <= 8 {
            // 直接解析小数部分
            let decimal = decimal_part.parse::<u64>().unwrap_or(0);
            let multiplier = 10_u64.pow(8 - decimal_len as u32);
            Ok(integer * PARSE_DECIMAL as u64 + decimal * multiplier)
        } else {
            // 超过8位小数，截断
            let truncated = &decimal_part[..8];
            let decimal = truncated.parse::<u64>().unwrap_or(0);
            Ok(integer * PARSE_DECIMAL as u64 + decimal)
        }
    } else {
        // 没有小数点，直接解析整数
        Ok(input.parse::<u64>().unwrap_or(0) * PARSE_DECIMAL as u64)
    }
}

#[inline]
pub fn s2u(input: &str) -> Result<u64, ParseFloatError> {
    input.parse::<f64>().map(|item|f2u(item))
}

#[inline]
pub fn s2u_decimal_fast(input: &str) -> Result<u64, ParseFloatError> {
    // 使用 Decimal 快速解析，避免精度损失
    let decimal = Decimal::from_str(input).unwrap_or(Decimal::ZERO);
    let mantissa = decimal.mantissa();
    let scale = decimal.scale();
    
    if scale <= 8 {
        let multiplier = 10_u64.pow(8 - scale as u32);
        Ok(mantissa as u64 * multiplier)
    } else {
        // 超过8位小数，截断到8位
        let truncated_multiplier = 10_u64.pow(8);
        Ok((mantissa as u64) / 10_u64.pow((scale - 8) as u32) * truncated_multiplier)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binance_signature_example() {
        // 示例来自币安官方文档
        // https://developers.binance.com/docs/binance-spot-api-docs/rest-api/endpoint-security-type
        let secret_key = "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
        let query_string = "symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";

        let expected_signature = "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71";

        let signature = generate_hmac_signature(query_string, secret_key);

        assert_eq!(signature, expected_signature);
    }

    #[test]
    fn test_align_price_precision() {
        // 测试BTC价格精度对齐（2位小数）
        let btc_market_price = 95000.12;
        let btc_stop_price = 94500.123456;
        let aligned_btc_stop = align_price_precision(btc_market_price, btc_stop_price);
        assert_eq!(aligned_btc_stop, 94500.12);

        // 测试ETH价格精度对齐（1位小数）
        let eth_market_price = 3200.5;
        let eth_stop_price = 3150.123456;
        let aligned_eth_stop = align_price_precision(eth_market_price, eth_stop_price);
        assert_eq!(aligned_eth_stop, 3150.1);

        // 测试SOL价格精度对齐（4位小数）
        let sol_market_price = 150.1234;
        let sol_stop_price = 145.123456;
        let aligned_sol_stop = align_price_precision(sol_market_price, sol_stop_price);
        assert_eq!(aligned_sol_stop, 145.1235);

        // 测试PEPE价格精度对齐（6位小数）
        let pepe_market_price = 0.000012;
        let pepe_stop_price = 0.000011234567;
        let aligned_pepe_stop = align_price_precision(pepe_market_price, pepe_stop_price);
        assert_eq!(aligned_pepe_stop, 0.000011);

        // 测试整数价格
        let int_market_price = 100.0;
        let int_stop_price = 95.123456;
        let aligned_int_stop = align_price_precision(int_market_price, int_stop_price);
        assert_eq!(aligned_int_stop, 95.0);
    }
}
