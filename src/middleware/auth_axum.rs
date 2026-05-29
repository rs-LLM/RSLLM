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
    response::{IntoResponse, Response},
};
// 用途：导入Deref和DerefMut trait
// 说明：用于JwtAuth结构体的解引用实现
use std::ops::{Deref, DerefMut};

// 用途：令牌头键名常量
// 说明：定义HTTP头中存储令牌的键名，统一使用"Authorization"
pub const TOKEN_KEY: http::header::HeaderName = http::header::AUTHORIZATION;

// 用途：Axum认证中间件
// 说明：用于验证请求中的JWT令牌，自动刷新即将过期的令牌
pub async fn auth(mut request: Request, next: Next) -> Result<Response, Response> {
    if let Ok(token) = get_token(request.headers()) {
        if let Some(token) = token_is_valid(token) {
            let now = rbatis::rbdc::DateTime::now().unix_timestamp() as usize;
            if (token.exp - now) < CONTEXT.config.jwt_refresh_token
                && let Ok(new_token) =
                    token.refresh(&CONTEXT.config.jwt_secret, CONTEXT.config.jwt_exp)
                && let Ok(new_header) = http::HeaderValue::from_str(&new_token)
            {
                request.headers_mut().insert(TOKEN_KEY, new_header);
            }
        } else {
            let error_response = axum::Json(serde_json::json!({
                "code": "401",
                "message": "无效的访问令牌，请重新登录",
                "data": null
            }));
            let mut response = error_response.into_response();
            *response.status_mut() = StatusCode::UNAUTHORIZED;
            return Ok(response);
        }
    } else {
        let error_response = axum::Json(serde_json::json!({
            "code": "401",
            "message": "缺少访问令牌，请先登录",
            "data": null
        }));
        let mut response = error_response.into_response();
        *response.status_mut() = StatusCode::UNAUTHORIZED;
        return Ok(response);
    }
    let response = next.run(request).await;
    Ok(response)
}

// 用途：验证令牌是否有效
// 说明：检查令牌的签名是否有效以及是否过期
fn token_is_valid(token: &str) -> Option<JWTToken> {
    checked_token(token).ok()
}

// 用途：从请求头中获取令牌
// 说明：提取Authorization头中的令牌字符串，去除Bearer 令牌前缀
fn get_token(h: &HeaderMap) -> Result<&str, Error> {
    Ok(h.get(TOKEN_KEY)
        .map(|v| v.to_str().unwrap_or_default())
        .unwrap_or_default()
        .trim_start_matches("Bearer "))
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
        // 说明：提取Authorization头中的令牌字符串，去除Bearer 令牌前缀
        if let Some(auth_header) = parts.headers.get(TOKEN_KEY) {
            if let Ok(auth_str) = auth_header.to_str() {
                // 用途：去除Bearer 令牌前缀
                // 说明：确保令牌格式正确
                let token = auth_str.trim_start_matches("Bearer ");
                // 用途：验证令牌
                // 说明：确保令牌的有效性
                match checked_token(token) {
                    Ok(v) => Ok(JwtAuth(v)),
                    Err(e) => {
                        let error_message = match e.to_string().as_str() {
                            "无效的访问令牌，请重新登录" => {
                                "无效的访问令牌，请重新登录".to_string()
                            }
                            "无效的令牌发行者，请重新登录" => {
                                "无效的令牌发行者，请重新登录".to_string()
                            }
                            "访问令牌已过期，请重新登录" => {
                                "访问令牌已过期，请重新登录".to_string()
                            }
                            _ => "令牌验证失败，请重新登录".to_string(),
                        };
                        Err((StatusCode::UNAUTHORIZED, error_message))
                    }
                }
            } else {
                Err((
                    StatusCode::UNAUTHORIZED,
                    "缺少访问令牌，请先登录".to_string(),
                ))
            }
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                "缺少访问令牌，请先登录".to_string(),
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
