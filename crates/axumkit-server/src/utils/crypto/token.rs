use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::Rng;

/// 암호학적으로 안전한 토큰 생성 (32바이트 = 256비트)
/// URL-safe Base64 인코딩으로 반환 (43자)
pub fn generate_secure_token() -> String {
    let token_bytes: [u8; 32] = rand::rng().random();
    URL_SAFE_NO_PAD.encode(token_bytes)
}

/// 지정된 길이의 암호학적으로 안전한 토큰 생성
pub fn generate_secure_token_with_length(byte_length: usize) -> String {
    let token_bytes: Vec<u8> = (0..byte_length).map(|_| rand::rng().random()).collect();
    URL_SAFE_NO_PAD.encode(&token_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_token() {
        let token = generate_secure_token();
        // 32 bytes -> 43 chars in base64 (no padding)
        assert_eq!(token.len(), 43);

        // 두 번 생성하면 다른 값
        let token2 = generate_secure_token();
        assert_ne!(token, token2);
    }

    #[test]
    fn test_generate_secure_token_with_length() {
        let token = generate_secure_token_with_length(16);
        // 16 bytes -> 22 chars in base64 (no padding)
        assert_eq!(token.len(), 22);
    }
}
