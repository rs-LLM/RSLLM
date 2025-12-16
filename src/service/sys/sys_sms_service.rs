// 用途：导入全局上下文
// 说明：用于访问配置和缓存服务
use crate::context::CONTEXT;

// 用途：导入短信表结构
// 说明：用于处理短信数据
use crate::domain::table::sms::Sms;

// 用途：导入自定义错误类型和结果类型
// 说明：用于统一错误处理
use crate::error::{Error, Result};

// 用途：导入错误信息宏
// 说明：用于生成错误信息
use crate::error_info;

// 用途：导入HashMap集合
// 说明：用于存储短信模板参数
use std::collections::HashMap;

/// 用途：系统短信服务
/// 说明：处理短信发送和验证码验证逻辑
pub struct SysSmsService {}

impl SysSmsService {
    /// 用途：发送验证码
    /// 说明：生成并发送短信验证码
    pub async fn send_verify_sms(&self, account: &str, sms_code: &str) -> Result<()> {
        // 用途：创建短信模板参数
        // 说明：存储短信类型和验证码
        let mut templete_arg = HashMap::new();
        // 用途：设置短信类型为验证码
        // 说明：标识当前短信的用途
        templete_arg.insert("sms_type".to_string(), "verify_sms".to_string());
        // 用途：设置验证码
        // 说明：将生成的验证码存储到模板参数中
        templete_arg.insert("sms_code".to_string(), sms_code.to_string());
        
        // 用途：将短信信息存入缓存
        // 说明：用于后续验证验证码
        let _r = CONTEXT
            .cache_service
            .set_json(
                // 用途：构建缓存键
                // 说明：使用前缀和账号构建唯一缓存键
                &format!("{},{}", CONTEXT.config.sms_cache_send_key_prefix, account),
                // 用途：创建短信对象
                // 说明：存储账号和模板参数
                &Sms {
                    account: account.to_string(),
                    args: templete_arg,
                },
            )
            .await?;
        
        // 用途：返回成功结果
        // 说明：告知调用者短信发送成功
        Ok(())
    }

    /// 用途：验证验证码
    /// 说明：检查用户输入的验证码是否正确
    pub async fn do_verify_sms(&self, account: &str, sms_code: &str) -> Result<bool> {
        // 用途：从缓存中获取短信信息
        // 说明：获取之前存储的验证码
        let sms: Option<Sms> = CONTEXT
            .cache_service
            .get_json(&format!(
                "{},{}",
                CONTEXT.config.sms_cache_send_key_prefix, account
            ))
            .await?;
        
        // 用途：匹配短信信息
        // 说明：处理短信存在和不存在的情况
        match sms {
            // 用途：处理短信存在的情况
            // 说明：验证用户输入的验证码是否与缓存中的一致
            Some(v) => {
                // 用途：获取缓存中的验证码
                // 说明：与用户输入的验证码进行比较
                let sms_code_cached = v.args.get("sms_code");
                // 用途：比较验证码
                // 说明：返回验证码是否匹配的结果
                Ok(sms_code_cached.eq(&Some(&sms_code.to_string())))
            }
            // 用途：处理短信不存在的情况
            // 说明：告知用户需要先发送验证码
            _ => Err(Error::from(error_info!("please_send_code"))),
        }
    }
}
