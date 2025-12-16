// 用途：密码编码器结构体
// 说明：封装密码编码和验证功能，提供统一的密码处理接口
pub struct PasswordEncoder {}

// 用途：PasswordEncoder实现
// 说明：提供密码编码和验证的具体实现
impl PasswordEncoder {
    // 用途：将原始密码编码为MD5哈希字符串
    // 说明：用于安全存储用户密码，防止明文存储
    pub fn encode(raw_password: &str) -> String {
        // 用途：计算MD5哈希值
        // 说明：MD5是一种常用的哈希算法，用于将任意长度的数据转换为固定长度的哈希值
        let digest = md5::compute(raw_password);
        // 用途：将哈希值格式化为十六进制字符串
        // 说明：方便存储和传输，十六进制字符串是人类可读的
        format!("{:x}", digest)
    }
    
    // 用途：验证密码是否匹配
    // 说明：用于登录验证，支持明文密码和哈希密码的验证
    pub fn verify(password: &str, raw_password: &str) -> bool {
        // 用途：检查密码是否与原始密码直接匹配
        // 说明：支持直接使用明文密码验证，便于测试和开发
        if password.eq(raw_password) {
            return true;
        }
        // 用途：将原始密码编码为哈希值
        // 说明：用于与存储的哈希密码进行比较
        let hashed = PasswordEncoder::encode(raw_password);
        // 用途：比较存储的密码与计算的哈希值
        // 说明：验证用户输入的密码是否正确
        password.eq(&hashed)
    }
}

// 用途：测试模块
// 说明：用于测试密码编码器的功能是否正常
#[cfg(test)]
mod test {
    // 用途：导入PasswordEncoder结构体
    // 说明：用于测试密码编码和验证功能
    use crate::util::password_encoder::PasswordEncoder;

    // 用途：测试编码功能
    // 说明：确保相同的密码生成相同的哈希值
    #[test]
    fn test_encode() {
        // 用途：编码密码
        // 说明：测试编码功能
        let s = PasswordEncoder::encode("123456");
        // 用途：打印编码结果
        // 说明：方便查看编码结果，用于调试
        println!("{}", s);
        // 用途：断言相同的密码生成相同的哈希值
        // 说明：验证编码功能的一致性
        assert_eq!(
            PasswordEncoder::encode("123456"),
            PasswordEncoder::encode("123456")
        )
    }

    // 用途：测试验证功能
    // 说明：确保能正确验证明文密码和哈希密码
    #[test]
    fn test_verify() {
        // 用途：定义测试密码
        // 说明：用于测试验证功能
        let password = "12345";
        let raw_password = "12345";

        // 用途：断言能正确验证明文密码
        // 说明：验证明文密码验证功能
        assert!(PasswordEncoder::verify(password, raw_password));

        // 用途：将密码编码为哈希值
        // 说明：用于测试哈希密码验证功能
        let encode_password = PasswordEncoder::encode(password);
        // 用途：断言能正确验证哈希密码
        // 说明：验证哈希密码验证功能
        assert!(PasswordEncoder::verify(&encode_password, password));
    }
}
