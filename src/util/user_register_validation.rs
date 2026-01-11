// 用途：用户注册验证工具模块
// 说明：提供邮箱格式验证和密码强度验证功能

// 用途：邮箱格式验证常量
// 说明：定义邮箱验证的最大长度限制
pub const MAX_EMAIL_LENGTH: usize = 255;

// 用途：密码强度验证常量
// 说明：定义密码验证的最小长度和强度要求
pub const MIN_PASSWORD_LENGTH: usize = 8;
pub const MAX_PASSWORD_LENGTH: usize = 128;

// 用途：用户注册验证器结构体
// 说明：提供邮箱和密码验证方法
pub struct UserRegisterValidator;

impl UserRegisterValidator {
    /// 验证邮箱格式
    ///
    /// # 规则
    /// - 不为空
    /// - 长度不超过255字符
    /// - 符合标准邮箱格式
    /// - 包含@符号
    /// - @前后都有内容
    /// - 域名部分包含至少一个点
    ///
    /// # 参数
    /// * `email` - 待验证的邮箱地址
    ///
    /// # 返回
    /// * `Result<(), String>` - 验证成功返回Ok，失败返回错误信息
    ///
    /// # 示例
    /// ```rust
    /// use rsllm::util::user_register_validation::UserRegisterValidator;
    /// assert!(UserRegisterValidator::validate_email("user@example.com").is_ok());
    /// assert!(UserRegisterValidator::validate_email("invalid-email").is_err());
    /// ```
    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.trim().is_empty() {
            return Err("邮箱不能为空".to_string());
        }

        if email.len() > MAX_EMAIL_LENGTH {
            return Err(format!("邮箱长度不能超过{}字符", MAX_EMAIL_LENGTH));
        }

        let email = email.trim();

        if !email.contains('@') {
            return Err("邮箱格式不正确：缺少@符号".to_string());
        }

        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err("邮箱格式不正确：@符号数量不正确".to_string());
        }

        let (local_part, domain_part) = (parts[0], parts[1]);

        if local_part.is_empty() {
            return Err("邮箱格式不正确：@前不能为空".to_string());
        }

        if domain_part.is_empty() {
            return Err("邮箱格式不正确：@后不能为空".to_string());
        }

        if !domain_part.contains('.') {
            return Err("邮箱格式不正确：域名部分必须包含点".to_string());
        }

        let domain_parts: Vec<&str> = domain_part.split('.').collect();
        if domain_parts.iter().any(|part| part.is_empty()) {
            return Err("邮箱格式不正确：域名部分格式不正确".to_string());
        }

        Ok(())
    }

    /// 验证密码强度
    ///
    /// # 规则
    /// - 不为空
    /// - 长度在8-128字符之间
    /// - 至少包含一个大写字母
    /// - 至少包含一个小写字母
    /// - 至少包含一个数字
    /// - 至少包含一个特殊字符（!@#$%^&*()_+-=[]{}|;:,.<>?）
    ///
    /// # 参数
    /// * `password` - 待验证的密码
    ///
    /// # 返回
    /// * `Result<(), String>` - 验证成功返回Ok，失败返回错误信息
    ///
    /// # 示例
    /// ```rust
    /// use rsllm::util::user_register_validation::UserRegisterValidator;
    /// assert!(UserRegisterValidator::validate_password("SecurePass123!").is_ok());
    /// assert!(UserRegisterValidator::validate_password("weak").is_err());
    /// ```
    pub fn validate_password(password: &str) -> Result<(), String> {
        if password.is_empty() {
            return Err("密码不能为空".to_string());
        }

        if password.len() < MIN_PASSWORD_LENGTH {
            return Err(format!("密码长度不能少于{}字符", MIN_PASSWORD_LENGTH));
        }

        if password.len() > MAX_PASSWORD_LENGTH {
            return Err(format!("密码长度不能超过{}字符", MAX_PASSWORD_LENGTH));
        }

        let has_uppercase = password.chars().any(|c| c.is_uppercase());
        let has_lowercase = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());
        let has_special = password
            .chars()
            .any(|c| "!@#$%^&*()_+-=[]{}|;:,.<>?".contains(c));

        if !has_uppercase {
            return Err("密码必须包含至少一个大写字母".to_string());
        }

        if !has_lowercase {
            return Err("密码必须包含至少一个小写字母".to_string());
        }

        if !has_digit {
            return Err("密码必须包含至少一个数字".to_string());
        }

        if !has_special {
            return Err("密码必须包含至少一个特殊字符（!@#$%^&*()_+-=[]{}|;:,.<>?）".to_string());
        }

        Ok(())
    }

    /// 验证用户名称
    ///
    /// # 规则
    /// - 不为空
    /// - 长度在1-50字符之间
    ///
    /// # 参数
    /// * `name` - 待验证的用户名称
    ///
    /// # 返回
    /// * `Result<(), String>` - 验证成功返回Ok，失败返回错误信息
    pub fn validate_name(name: &str) -> Result<(), String> {
        let name = name.trim();

        if name.is_empty() {
            return Err("用户名称不能为空".to_string());
        }

        if name.len() > 50 {
            return Err("用户名称长度不能超过50字符".to_string());
        }

        Ok(())
    }

    /// 验证服务条款同意状态
    ///
    /// # 规则
    /// - 必须为true
    ///
    /// # 参数
    /// * `agree_terms` - 是否同意服务条款
    ///
    /// # 返回
    /// * `Result<(), String>` - 验证成功返回Ok，失败返回错误信息
    pub fn validate_agree_terms(agree_terms: bool) -> Result<(), String> {
        if !agree_terms {
            return Err("必须同意服务条款和隐私政策才能注册".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        assert!(UserRegisterValidator::validate_email("user@example.com").is_ok());
        assert!(UserRegisterValidator::validate_email("test.user@example.com").is_ok());
        assert!(UserRegisterValidator::validate_email("user123@test.example.com").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(UserRegisterValidator::validate_email("").is_err());
        assert!(UserRegisterValidator::validate_email("   ").is_err());
        assert!(UserRegisterValidator::validate_email("invalid-email").is_err());
        assert!(UserRegisterValidator::validate_email("@example.com").is_err());
        assert!(UserRegisterValidator::validate_email("user@").is_err());
        assert!(UserRegisterValidator::validate_email("user@com").is_err());
    }

    #[test]
    fn test_validate_password_valid() {
        assert!(UserRegisterValidator::validate_password("SecurePass123!").is_ok());
        assert!(UserRegisterValidator::validate_password("MyP@ssw0rd").is_ok());
        assert!(UserRegisterValidator::validate_password("Test1234@").is_ok());
    }

    #[test]
    fn test_validate_password_invalid() {
        assert!(UserRegisterValidator::validate_password("").is_err());
        assert!(UserRegisterValidator::validate_password("weak").is_err());
        assert!(UserRegisterValidator::validate_password("weakpassword").is_err());
        assert!(UserRegisterValidator::validate_password("WEAKPASSWORD").is_err());
        assert!(UserRegisterValidator::validate_password("weakpassword1").is_err());
        assert!(UserRegisterValidator::validate_password("WEAKPASSWORD1").is_err());
        assert!(UserRegisterValidator::validate_password("Weakpassword").is_err());
        assert!(UserRegisterValidator::validate_password("Weak12345").is_err());
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(UserRegisterValidator::validate_name("张三").is_ok());
        assert!(UserRegisterValidator::validate_name("John Doe").is_ok());
        assert!(UserRegisterValidator::validate_name("TestUser").is_ok());
    }

    #[test]
    fn test_validate_name_invalid() {
        assert!(UserRegisterValidator::validate_name("").is_err());
        assert!(UserRegisterValidator::validate_name("   ").is_err());
    }

    #[test]
    fn test_validate_agree_terms() {
        assert!(UserRegisterValidator::validate_agree_terms(true).is_ok());
        assert!(UserRegisterValidator::validate_agree_terms(false).is_err());
    }
}
