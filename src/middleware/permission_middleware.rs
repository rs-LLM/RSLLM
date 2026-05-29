use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::context::CONTEXT;
use crate::domain::vo::JWTToken;
use crate::error::Error;
use crate::middleware::{auth::checked_token, auth_axum::TOKEN_KEY};

pub fn require_permission(
    required_permission: &'static str,
) -> impl Fn(Request, Next) -> futures::future::BoxFuture<'static, Result<Response, Response>>
+ Clone
+ Send
+ Sync
+ 'static {
    move |request: Request, next: Next| {
        let permission = required_permission;
        Box::pin(async move {
            if let Ok(token) = get_token(request.headers())
                && let Some(jwt_token) = token_is_valid(token)
            {
                if has_permission(&jwt_token, permission).await {
                    let response = next.run(request).await;
                    return Ok(response);
                } else {
                    return Err(create_permission_error(
                        "权限不足，需要".to_string() + permission + "权限",
                    ));
                }
            }
            Err(create_permission_error("无效的访问令牌".to_string()))
        })
    }
}

async fn has_permission(token: &JWTToken, required_permission: &str) -> bool {
    log::debug!(
        "检查权限: 需要权限={}, 用户权限列表={:?}",
        required_permission,
        token.permissions
    );

    let perms = match CONTEXT
        .sys_user_service
        .load_level_permission(token.id.as_str())
        .await
    {
        Ok(v) => v,
        Err(e) => {
            log::warn!(
                "权限查询失败，将退回使用JWT权限: user_id={}, err={}",
                token.id,
                e
            );
            token.permissions.clone()
        }
    };

    let result = perms.contains(&required_permission.to_string());
    log::debug!("权限检查结果: {}", result);
    result
}

fn token_is_valid(token: &str) -> Option<JWTToken> {
    checked_token(token).ok()
}

fn get_token(h: &HeaderMap) -> Result<&str, Error> {
    Ok(h.get(TOKEN_KEY)
        .map(|v| v.to_str().unwrap_or_default())
        .unwrap_or_default()
        .trim_start_matches("Bearer "))
}

fn create_permission_error(message: String) -> Response {
    let error_response = axum::Json(serde_json::json!({
        "code": "403",
        "msg": message,
        "data": null
    }));
    let mut response = error_response.into_response();
    *response.status_mut() = StatusCode::FORBIDDEN;
    response
}
