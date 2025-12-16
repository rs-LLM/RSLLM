// 用途：导入系统认证DTO
// 说明：用于接收认证检查请求参数
use crate::domain::dto::auth::SysAuthDTO;

// 用途：导入JWT令牌VO
// 说明：用于返回认证结果
use crate::domain::vo::JWTToken;

// 用途：导入自定义结果类型
// 说明：用于统一错误处理
use crate::error::Result;

/// 用途：系统认证服务
/// 说明：处理系统认证相关业务逻辑
pub struct SysAuthService {}

impl SysAuthService {
    /// 用途：检查用户是否有权限访问指定路径
    /// 说明：验证用户令牌和请求路径的权限关系
    pub async fn check_auth(&self, arg: SysAuthDTO) -> Result<JWTToken> {
        // 用途：验证令牌有效性
        // 说明：确保用户提供的令牌是有效的
        let jwt = crate::middleware::auth::checked_token(&arg.access_token)?;
        
        // 用途：检查权限
        // 说明：验证用户是否有权限访问请求的路径
        crate::middleware::auth::check_auth(&jwt, &arg.path).await?;
        
        // 用途：返回验证后的JWT令牌
        // 说明：将验证结果返回给调用者
        Ok(jwt)
    }
}
