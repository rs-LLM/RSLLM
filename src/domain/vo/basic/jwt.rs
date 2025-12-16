// 用途：导入自定义错误类型
// 说明：用于JWT操作中的错误处理
use crate::error::Error;
// 用途：导入JWT错误类型
// 说明：用于JWT验证时的错误匹配
use jsonwebtoken::errors::ErrorKind;
// 用途：导入JWT相关结构体和函数
// 说明：用于生成和验证JWT令牌
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
// 用途：导入serde的序列化和反序列化特性
// 说明：支持JWTToken的JSON序列化和反序列化，便于在网络中传输
use serde::{Deserialize, Serialize};

// 用途：JWT认证令牌结构体
// 说明：用于存储JWT令牌的声明信息，包含用户身份和权限
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq,Default)]
pub struct JWTToken {
    // 用途：用户ID
    // 说明：标识令牌所属的用户
    pub id: String,
    // 用途：用户账号
    // 说明：存储用户登录账号
    pub account: String,
    // 用途：用户权限列表
    // 说明：存储用户拥有的权限标识，用于权限验证
    pub permissions: Vec<String>,
    // 用途：用户角色ID列表
    // 说明：存储用户拥有的角色ID，用于角色验证
    pub role_ids: Vec<String>,
    // 用途：过期时间
    // 说明：标识令牌的有效期，防止令牌被永久使用
    pub exp: usize,
}

// 用途：JWTToken实现
// 说明：提供JWT令牌的生成、验证和刷新方法
impl JWTToken {
    // 用途：创建JWT令牌
    // 说明：将用户信息和权限编码为JWT令牌，用于身份验证
    // secret: JWT签名密钥，用于保证令牌的完整性和真实性
    pub fn create_token(&self, secret: &str) -> Result<String, Error> {
        match encode(
            &Header::default(),
            self,
            &EncodingKey::from_secret(secret.as_ref()),
        ) {
            Ok(t) => Ok(t),
            Err(_) => Err(Error::from("JWTToken encode fail!")),
        }
    }
    
    // 用途：验证JWT令牌的有效性
    // 说明：检查令牌是否被篡改、是否过期等，确保令牌的合法性
    // secret: JWT签名密钥，用于验证令牌签名
    pub fn verify(secret: &str, token: &str) -> Result<JWTToken, Error> {
        let mut validation = Validation::default();
        validation.leeway = 0;
        match decode::<JWTToken>(
            &token,
            &DecodingKey::from_secret(secret.as_ref()),
            &validation,
        ) {
            Ok(c) => Ok(c.claims),
            Err(err) => match *err.kind() {
                ErrorKind::InvalidToken => return Err(Error::from("InvalidToken")),
                ErrorKind::InvalidIssuer => return Err(Error::from("InvalidIssuer")),
                ErrorKind::ExpiredSignature => return Err(Error::from("ExpiredSignature")),
                _ => return Err(Error::from("InvalidToken other errors")),
            },
        }
    }

    // 用途：刷新JWT令牌
    // 说明：延长令牌的有效期，避免用户频繁重新登录
    // secret: JWT签名密钥
    // jwt_exp: 令牌过期时间，单位为秒
    pub fn refresh(&self, secret: &str, jwt_exp: usize) -> Result<String, Error> {
        let mut jwt = self.clone();
        jwt.exp = jwt.exp + jwt_exp;
        jwt.create_token(&secret)
    }
}

// 用途：测试模块
// 说明：用于测试JWTToken的功能是否正常
#[cfg(test)]
mod test {
    // 用途：导入JWTToken结构体
    // 说明：用于测试JWTToken的功能
    use crate::domain::vo::JWTToken;
    // 用途：导入日期时间类型
    // 说明：用于设置令牌的过期时间
    use rbatis::rbdc::types::DateTime;
    // 用途：导入睡眠函数
    // 说明：用于测试令牌的过期时间
    use std::thread::sleep;
    // 用途：导入时间间隔类型
    // 说明：用于设置睡眠时长
    use std::time::Duration;
    // 用途：导入自定义错误类型
    // 说明：用于测试函数的错误处理
    use crate::error::Error;

    // 用途：测试JWT功能
    // 说明：验证JWTToken的生成、验证和刷新功能是否正常
    #[test]
    fn test_jwt() -> Result<(),Error>{
        let j = JWTToken {
            id: "1".to_string(),
            account: "189".to_string(),
            permissions: vec![],
            role_ids: vec![],
            exp: DateTime::now().unix_timestamp() as usize + 10,
        };
        sleep(Duration::from_secs(5));
        let token = j.create_token("ssss")?;
        assert_eq!(JWTToken::verify("ssss", &token)?, j);
        Ok(())
    }
}
