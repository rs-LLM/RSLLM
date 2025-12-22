//! 安全加密服务模块
//! 提供AES-256-GCM加密、解密和哈希功能

use aes_gcm::{
    aead::{Aead, KeyInit, generic_array::GenericArray},
    Aes256Gcm, Nonce
};
use rand::Rng;
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use crate::error::{Result, Error};

/// 加密服务
///
/// 提供AES-256-GCM加密、解密和SHA256哈希功能
#[derive(Clone)]
pub struct EncryptionService {
    cipher: Aes256Gcm,
    _key: Vec<u8>, // 保留密钥用于未来扩展，目前未使用
}

impl EncryptionService {
    /// 创建加密服务实例
    ///
    /// # Arguments
    ///
    /// * `key` - 32字节的加密密钥
    ///
    /// # Returns
    ///
    /// * `Result<Self>` - 加密服务实例或错误
    ///
    /// # Example
    ///
    /// ```
    /// let key = [0u8; 32]; // 32字节密钥
    /// let service = EncryptionService::new(&key)?;
    /// ```
    pub fn new(key: &[u8]) -> Result<Self> {
        if key.len() != 32 {
            return Err(Error::ConfigError("Encryption key must be 32 bytes".to_string()));
        }
        
        let key_array = GenericArray::from_slice(key);
        let cipher = Aes256Gcm::new(key_array);
        
        Ok(Self {
            cipher,
            _key: key.to_vec(),
        })
    }
    
    /// 加密字符串
    ///
    /// 使用AES-256-GCM算法加密数据，返回base64编码的字符串
    ///
    /// # Arguments
    ///
    /// * `plaintext` - 要加密的明文字符串
    ///
    /// # Returns
    ///
    /// * `Result<String>` - 加密后的base64字符串或错误
    ///
    /// # Example
    ///
    /// ```
    /// let encrypted = service.encrypt("secret data")?;
    /// ```
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        // 生成随机nonce (12字节)
        let nonce_bytes = rand::thread_rng().r#gen::<[u8; 12]>();
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // 加密数据
        let ciphertext = self.cipher.encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| Error::EncryptionError(e.to_string()))?;
        
        // 组合nonce和ciphertext
        let mut combined = Vec::with_capacity(12 + ciphertext.len());
        combined.extend_from_slice(nonce);
        combined.extend_from_slice(&ciphertext);
        
        // base64编码
        Ok(general_purpose::STANDARD.encode(combined))
    }
    
    /// 解密字符串
    ///
    /// 解密base64编码的加密数据
    ///
    /// # Arguments
    ///
    /// * `encrypted` - base64编码的加密字符串
    ///
    /// # Returns
    ///
    /// * `Result<String>` - 解密后的明文或错误
    ///
    /// # Example
    ///
    /// ```
    /// let decrypted = service.decrypt(encrypted)?;
    /// ```
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        // base64解码
        let combined = general_purpose::STANDARD.decode(encrypted)
            .map_err(|e| Error::EncryptionError(e.to_string()))?;
        
        if combined.len() < 12 {
            return Err(Error::EncryptionError("Invalid encrypted data".to_string()));
        }
        
        // 分离nonce和ciphertext
        let nonce = Nonce::from_slice(&combined[..12]);
        let ciphertext = &combined[12..];
        
        // 解密数据
        let plaintext = self.cipher.decrypt(nonce, ciphertext)
            .map_err(|e| Error::EncryptionError(e.to_string()))?;
        
        // 转换为字符串
        String::from_utf8(plaintext)
            .map_err(|e| Error::EncryptionError(e.to_string()))
    }
    
    /// 计算SHA256哈希
    ///
    /// 计算数据的SHA256哈希值，用于密钥查找和验证
    ///
    /// # Arguments
    ///
    /// * `data` - 要计算哈希的数据
    ///
    /// # Returns
    ///
    /// * `String` - 64字符的十六进制哈希值
    ///
    /// # Example
    ///
    /// ```
    /// let hash = service.hash("data");
    /// ```
    pub fn hash(&self, data: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        format!("{:x}", hasher.finalize())
    }
    
    /// 验证哈希
    ///
    /// 验证数据与哈希值是否匹配
    ///
    /// # Arguments
    ///
    /// * `data` - 原始数据
    /// * `hash` - 期望的哈希值
    ///
    /// # Returns
    ///
    /// * `bool` - 是否匹配
    ///
    /// # Example
    ///
    /// ```
    /// let is_valid = service.verify_hash("data", "expected_hash");
    /// ```
    pub fn verify_hash(&self, data: &str, hash: &str) -> bool {
        self.hash(data) == hash
    }
    
    /// 生成安全随机字符串
    ///
    /// 生成指定长度的安全随机字符串
    ///
    /// # Arguments
    ///
    /// * `length` - 要生成的字符串长度
    ///
    /// # Returns
    ///
    /// * `String` - 随机字符串
    ///
    /// # Example
    ///
    /// ```
    /// let random_str = service.generate_random(32);
    /// ```
    pub fn generate_random(&self, length: usize) -> String {
        let charset: Vec<u8> = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
            .to_vec();
        
        let mut rng = rand::thread_rng();
        (0..length)
            .map(|_| charset[rng.gen_range(0..charset.len())] as char)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let key = [0u8; 32];
        let service = EncryptionService::new(&key)?;
        
        let original = "Hello, World!";
        let encrypted = service.encrypt(original)?;
        let decrypted = service.decrypt(&encrypted)?;
        
        assert_eq!(original, decrypted);
        Ok(())
    }

    #[test]
    fn test_hash() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let key = [0u8; 32];
        let service = EncryptionService::new(&key)?;
        
        let hash1 = service.hash("test");
        let hash2 = service.hash("test");
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex string length
        Ok(())
    }

    #[test]
    fn test_verify_hash() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let key = [0u8; 32];
        let service = EncryptionService::new(&key)?;
        
        let data = "test data";
        let hash = service.hash(data);
        
        assert!(service.verify_hash(data, &hash));
        assert!(!service.verify_hash("different data", &hash));
        Ok(())
    }

    #[test]
    fn test_generate_random() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let key = [0u8; 32];
        let service = EncryptionService::new(&key)?;
        
        let random1 = service.generate_random(32);
        let random2 = service.generate_random(32);
        
        assert_eq!(random1.len(), 32);
        assert_eq!(random2.len(), 32);
        assert_ne!(random1, random2); // Should be different
        Ok(())
    }

    #[test]
    fn test_invalid_key_length() {
        let result = EncryptionService::new(&[0u8; 16]); // Wrong length
        assert!(result.is_err());
    }
}