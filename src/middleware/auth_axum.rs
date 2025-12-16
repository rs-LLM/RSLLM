// 用途：导入全局上下文实例
// 说明：用于访问配置信息和服务实例
use crate::context::CONTEXT;
// 用途：导入JWT令牌结构体
// 说明：用于令牌验证和自动刷新
use crate::domain::vo::JWTToken;
// 用途：导入自定义错误类型
// 说明：用于错误处理
use crate::error::Error;
// 用途：导入令牌验证函数
// 说明：用于验证JWT令牌的有效性
use crate::middleware::auth::checked_token;
// 用途：导入FromRequestParts trait
// 说明：用于从请求中提取JWT令牌
use axum::extract::FromRequestParts;
// 用途：导入请求部件类型
// 说明：用于从请求部件中提取令牌
use axum::http::request::Parts;
// 用途：导入axum相关类型
// 说明：用于实现中间件和请求处理
use axum::{
    extract::Request,
    http,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
// 用途：导入Deref和DerefMut trait
// 说明：用于JwtAuth结构体的解引用实现
use std::ops::{Deref, DerefMut};

// 用途：令牌头键名常量
// 说明：定义HTTP头中存储令牌的键名，统一使用"Authorization"
pub const TOKEN_KEY: &'static str = "Authorization";

// 用途：Axum认证中间件
// 说明：用于验证请求中的JWT令牌，自动刷新即将过期的令牌
pub async fn auth(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    // 用途：检查是否为调试模式
    // 说明：调试模式下跳过认证，方便开发和测试
    if !CONTEXT.config.debug() {
        // 用途：从请求头中获取令牌
        // 说明：验证请求是否包含有效的令牌
        if let Ok(token) = get_token(&request.headers()) {
            // 用途：验证令牌是否有效
            // 说明：确保只有有效的令牌才能访问受保护的资源
            if let Some(token) = token_is_valid(&token) {
                // 用途：获取当前时间
                // 说明：用于判断令牌是否即将过期
                let now = rbatis::rbdc::DateTime::now().unix_timestamp() as usize;
                // 用途：检查令牌是否即将过期
                // 说明：如果令牌即将过期，自动刷新令牌，避免用户频繁重新登录
                if (token.exp - now) < CONTEXT.config.jwt_refresh_token {
                    // 用途：刷新令牌
                    // 说明：生成新的令牌，延长有效期
                    if let Ok(new_token) =
                        token.refresh(&CONTEXT.config.jwt_secret, CONTEXT.config.jwt_exp)
                    {
                        // 用途：将新令牌添加到请求头中
                        // 说明：方便后续中间件或处理函数使用新令牌
                        if let Ok(new_header) = http::HeaderValue::from_str(&new_token) {
                            request.headers_mut().insert(TOKEN_KEY, new_header);
                        }
                    }
                }
            } else {
                // 用途：返回未授权状态
                // 说明：令牌无效时拒绝请求
                return Err(StatusCode::UNAUTHORIZED);
            }
        } else {
            // 用途：返回未授权状态
            // 说明：请求头中没有令牌时拒绝请求
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    // 用途：继续处理请求
    // 说明：令牌验证通过后，将请求传递给下一个中间件或处理函数
    let response = next.run(request).await;
    Ok(response)
}

// 用途：验证令牌是否有效
// 说明：检查令牌的签名是否有效以及是否过期
fn token_is_valid(token: &str) -> Option<JWTToken> {
    match checked_token(token) {
        Ok(data) => Some(data),
        Err(_) => None,
    }
}

// 用途：从请求头中获取令牌
// 说明：提取Authorization头中的令牌字符串
fn get_token(h: &HeaderMap) -> Result<&str, Error> {
    Ok(h.get(TOKEN_KEY)
        .map(|v| v.to_str().unwrap_or_default())
        .unwrap_or_default())
}

// 用途：Axum JWT认证提取器
// 说明：用于从请求中提取JWT令牌，方便处理函数直接使用
pub struct JwtAuth(pub JWTToken);

// 用途：为JwtAuth实现Deref trait
// 说明：允许直接访问内部的JWTToken，简化代码
impl Deref for JwtAuth {
    type Target = JWTToken;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// 用途：为JwtAuth实现DerefMut trait
// 说明：允许直接修改内部的JWTToken，简化代码
impl DerefMut for JwtAuth {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

// 用途：为JwtAuth实现FromRequestParts trait
// 说明：允许Axum从请求中自动提取JWT令牌，注入到处理函数中
impl<S: Sync> FromRequestParts<S> for JwtAuth {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 用途：从请求头中获取令牌
        // 说明：提取Authorization头中的令牌字符串
        if let Some(auth_header) = parts.headers.get(TOKEN_KEY) {
            if let Ok(auth_str) = auth_header.to_str() {
                // 用途：验证令牌
                // 说明：确保令牌的有效性
                match checked_token(auth_str) {
                    Ok(v) => Ok(JwtAuth(v)),
                    Err(e) => Err((
                        StatusCode::UNAUTHORIZED,
                        format!("Invalid authorization header={}", e),
                    )),
                }
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    "Invalid authorization header".to_string(),
                ))
            }
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                "Authorization header missing".to_string(),
            ))
        }
    }
}

// 用途：实现JwtAuth到JWTToken的转换
// 说明：方便将JwtAuth转换为JWTToken，简化代码
impl From<JwtAuth> for JWTToken {
    fn from(jwt: JwtAuth) -> Self {
        JWTToken {
            id: jwt.id.clone(),
            account: jwt.account.clone(),
            permissions: jwt.permissions.clone(),
            role_ids: jwt.role_ids.clone(),
            exp: jwt.exp,
        }
    }
}
