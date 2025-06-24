use hmac::{Hmac, Mac};
use sha2::Sha256;

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
    
    let mut mac = HmacSha256::new_from_slice(secret_key.as_bytes())
        .expect("HMAC can take a key of any size");
    
    mac.update(query_string.as_bytes());
    
    let result = mac.finalize();
    let code_bytes = result.into_bytes();
    
    hex::encode(code_bytes)
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
}
