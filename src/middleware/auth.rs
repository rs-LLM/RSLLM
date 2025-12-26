// 用途：导入全局上下文实例
// 说明：用于访问配置信息和服务实例
use crate::context::CONTEXT;
// 用途：导入JWT令牌结构体
// 说明：用于令牌验证和权限检查
use crate::domain::vo::JWTToken;

// 用途：认证中间件结构体
// 说明：作为认证相关方法的命名空间
pub struct Auth;

// 用途：检查令牌的有效性和过期时间
// 说明：确保只有有效的令牌才能访问受保护的资源
pub fn checked_token(token: &str) -> Result<JWTToken, crate::error::Error> {
    // 用途：验证令牌
    // 说明：检查令牌的签名是否有效以及是否过期
    let token = JWTToken::verify(&CONTEXT.config.jwt_secret, token);
    match token {
        Ok(token) => Ok(token),
        Err(e) => Err(crate::error::Error::from(e.to_string())),
    }
}

// 用途：权限检查函数
// 说明：检查用户是否有权限访问特定路径
pub async fn check_auth(token: &JWTToken, path: &str) -> Result<(), crate::error::Error> {
    // 用途：获取所有系统权限
    // 说明：用于与用户令牌中的权限进行比对
    let sys_permission = CONTEXT.rbac_permission_service.finds_all().await?;
    // 用途：遍历用户令牌中的权限
    // 说明：检查用户是否拥有访问指定路径的权限
    for token_permission in &token.permissions {
        // 用途：遍历系统中定义的所有权限
        // 说明：查找与用户权限匹配的系统权限
        for x in &sys_permission {
            match &x.permission {
                Some(permission) => match &x.path {
                    None => {}
                    Some(x_path) => {
                        // 用途：检查权限和路径是否匹配
                        // 说明：如果匹配则允许访问，否则继续检查
                        if permission.eq(token_permission) && path.contains(x_path) {
                            return Ok(());
                        }
                    }
                },
                _ => {}
            }
        }
    }
    // 用途：返回权限拒绝错误
    // 说明：如果没有匹配的权限，拒绝访问请求，返回详细的错误信息
    let error_message = format!("您没有权限访问该资源，所需路径: {}", path);
    Err(crate::error::Error::AuthError(error_message))
}
