//! AI Hub 输入验证模块
//!
//! 提供统一的输入参数验证功能，确保请求数据的有效性和安全性

/// 验证结果类型
pub type ValidationResult<T> = Result<T, String>;

/// 模型名称验证常量
pub const MAX_MODEL_NAME_LENGTH: usize = 255;
pub const MIN_MODEL_NAME_LENGTH: usize = 1;

/// 消息验证常量
pub const MAX_MESSAGES: usize = 100;
pub const MIN_MESSAGES: usize = 1;

/// Token限制常量
pub const MAX_TOKENS: u32 = 4096;
pub const MIN_TOKENS: u32 = 1;

/// 温度参数常量
pub const MAX_TEMPERATURE: f32 = 2.0;
pub const MIN_TEMPERATURE: f32 = 0.0;

/// Top P参数常量
pub const MAX_TOP_P: f32 = 1.0;
pub const MIN_TOP_P: f32 = 0.0;

/// 频率/存在惩罚参数常量
pub const MAX_PENALTY: f32 = 2.0;
pub const MIN_PENALTY: f32 = -2.0;

/// 生成数量常量
pub const MAX_N: u32 = 20;
pub const MIN_N: u32 = 1;

/// 嵌入输入验证常量
pub const MAX_EMBEDDING_INPUTS: usize = 2048;
pub const MAX_TEXT_LENGTH: usize = 10000;

/// 聊天补全请求参数
pub struct ChatCompletionRequestParams<'a> {
    pub model: &'a str,
    pub messages: &'a [super::content::ChatCompletionMessage],
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub n: Option<u32>,
}

/// 文本补全请求参数
pub struct CompletionRequestParams<'a> {
    pub model: &'a str,
    pub prompt: &'a str,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub n: Option<u32>,
}

/// 验证器结构体，提供各种验证方法
pub struct Validator;

impl Validator {
    /// 验证模型名称
    ///
    /// # 规则
    /// - 不为空
    /// - 长度在1-255字符之间
    /// - 不包含特殊字符（只允许字母、数字、下划线、连字符、点）
    pub fn validate_model_name(model: &str) -> ValidationResult<()> {
        if model.trim().is_empty() {
            return Err("模型名称不能为空".to_string());
        }

        if model.len() < MIN_MODEL_NAME_LENGTH || model.len() > MAX_MODEL_NAME_LENGTH {
            return Err(format!(
                "模型名称长度必须在{}-{}字符之间",
                MIN_MODEL_NAME_LENGTH, MAX_MODEL_NAME_LENGTH
            ));
        }

        // 检查特殊字符（允许字母、数字、下划线、连字符、点、斜杠）
        if !model
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/')
        {
            return Err("模型名称只能包含字母、数字、下划线、连字符、点和斜杠".to_string());
        }

        Ok(())
    }

    /// 验证消息列表
    ///
    /// # 规则
    /// - 不为空
    /// - 数量不超过限制
    /// - 每条消息都有有效的角色和内容
    pub fn validate_messages(
        messages: &[super::content::ChatCompletionMessage],
    ) -> ValidationResult<()> {
        if messages.is_empty() {
            return Err("消息列表不能为空".to_string());
        }

        if messages.len() > MAX_MESSAGES {
            return Err(format!("消息数量不能超过{}", MAX_MESSAGES));
        }

        for (i, message) in messages.iter().enumerate() {
            // 验证角色
            if message.role.trim().is_empty() {
                return Err(format!("消息{}的角色不能为空", i));
            }

            // 验证内容
            match &message.content {
                Some(content) => match content {
                    super::content::ChatMessageContent::String(s) => {
                        if s.trim().is_empty() {
                            return Err(format!("消息{}的内容不能为空", i));
                        }
                        if s.len() > MAX_TEXT_LENGTH {
                            return Err(format!("消息{}的内容过长", i));
                        }
                    }
                    super::content::ChatMessageContent::Array(parts) => {
                        if parts.is_empty() {
                            return Err(format!("消息{}的内容不能为空", i));
                        }
                        for part in parts {
                            if let Some(text) = &part.text {
                                if text.trim().is_empty() {
                                    return Err(format!("消息{}的文本内容不能为空", i));
                                }
                                if text.len() > MAX_TEXT_LENGTH {
                                    return Err(format!("消息{}的文本内容过长", i));
                                }
                            }
                        }
                    }
                },
                None => {
                    return Err(format!("消息{}的内容不能为空", i));
                }
            }
        }

        Ok(())
    }

    /// 验证max_tokens参数
    ///
    /// # 规则
    /// - 必须在1-4096范围内
    pub fn validate_max_tokens(max_tokens: Option<u32>) -> ValidationResult<()> {
        if let Some(tokens) = max_tokens
            && !(MIN_TOKENS..=MAX_TOKENS).contains(&tokens)
        {
            return Err(format!(
                "max_tokens必须在{}-{}范围内",
                MIN_TOKENS, MAX_TOKENS
            ));
        }
        Ok(())
    }

    /// 验证temperature参数
    ///
    /// # 规则
    /// - 必须在0.0-2.0范围内
    pub fn validate_temperature(temperature: Option<f32>) -> ValidationResult<()> {
        if let Some(temp) = temperature
            && !(MIN_TEMPERATURE..=MAX_TEMPERATURE).contains(&temp)
        {
            return Err(format!(
                "temperature必须在{:.1}-{:.1}范围内",
                MIN_TEMPERATURE, MAX_TEMPERATURE
            ));
        }
        Ok(())
    }

    /// 验证top_p参数
    ///
    /// # 规则
    /// - 必须在0.0-1.0范围内
    pub fn validate_top_p(top_p: Option<f32>) -> ValidationResult<()> {
        if let Some(p) = top_p
            && !(MIN_TOP_P..=MAX_TOP_P).contains(&p)
        {
            return Err(format!(
                "top_p必须在{:.1}-{:.1}范围内",
                MIN_TOP_P, MAX_TOP_P
            ));
        }
        Ok(())
    }

    /// 验证频率惩罚参数
    ///
    /// # 规则
    /// - 必须在-2.0-2.0范围内
    pub fn validate_frequency_penalty(frequency_penalty: Option<f32>) -> ValidationResult<()> {
        if let Some(penalty) = frequency_penalty
            && (!(MIN_PENALTY..=MAX_PENALTY).contains(&penalty))
        {
            return Err(format!(
                "frequency_penalty必须在{:.1}-{:.1}范围内",
                MIN_PENALTY, MAX_PENALTY
            ));
        }
        Ok(())
    }

    /// 验证存在惩罚参数
    ///
    /// # 规则
    /// - 必须在-2.0-2.0范围内
    pub fn validate_presence_penalty(presence_penalty: Option<f32>) -> ValidationResult<()> {
        if let Some(penalty) = presence_penalty
            && (!(MIN_PENALTY..=MAX_PENALTY).contains(&penalty))
        {
            return Err(format!(
                "presence_penalty必须在{:.1}-{:.1}范围内",
                MIN_PENALTY, MAX_PENALTY
            ));
        }
        Ok(())
    }

    /// 验证生成数量参数
    ///
    /// # 规则
    /// - 必须在1-20范围内
    pub fn validate_n(n: Option<u32>) -> ValidationResult<()> {
        if let Some(count) = n
            && !(MIN_N..=MAX_N).contains(&count)
        {
            return Err(format!("n必须在{}-{}范围内", MIN_N, MAX_N));
        }
        Ok(())
    }

    /// 验证文本补全的prompt
    ///
    /// # 规则
    /// - 不为空
    /// - 长度不超过限制
    pub fn validate_prompt(prompt: &str) -> ValidationResult<()> {
        if prompt.trim().is_empty() {
            return Err("prompt不能为空".to_string());
        }

        if prompt.len() > MAX_TEXT_LENGTH {
            return Err(format!("prompt长度不能超过{}字符", MAX_TEXT_LENGTH));
        }

        Ok(())
    }

    /// 验证嵌入输入
    ///
    /// # 规则
    /// - 不为空
    /// - 输入数量不超过限制
    /// - 每个文本长度不超过限制
    pub fn validate_embedding_input(
        input: &super::embeddings::EmbeddingsInput,
    ) -> ValidationResult<()> {
        match input {
            super::embeddings::EmbeddingsInput::Single(text) => {
                if text.trim().is_empty() {
                    return Err("嵌入输入不能为空".to_string());
                }
                if text.len() > MAX_TEXT_LENGTH {
                    return Err(format!("嵌入输入文本长度不能超过{}字符", MAX_TEXT_LENGTH));
                }
            }
            super::embeddings::EmbeddingsInput::Multiple(texts) => {
                if texts.is_empty() {
                    return Err("嵌入输入列表不能为空".to_string());
                }
                if texts.len() > MAX_EMBEDDING_INPUTS {
                    return Err(format!("嵌入输入数量不能超过{}", MAX_EMBEDDING_INPUTS));
                }
                for (i, text) in texts.iter().enumerate() {
                    if text.trim().is_empty() {
                        return Err(format!("嵌入输入{}不能为空", i));
                    }
                    if text.len() > MAX_TEXT_LENGTH {
                        return Err(format!("嵌入输入{}文本长度过长", i));
                    }
                }
            }
            super::embeddings::EmbeddingsInput::SingleTokenIds(token_ids) => {
                if token_ids.is_empty() {
                    return Err("嵌入token ID列表不能为空".to_string());
                }
                if token_ids.len() > MAX_TEXT_LENGTH {
                    return Err(format!("嵌入token ID数量不能超过{}", MAX_TEXT_LENGTH));
                }
            }
            super::embeddings::EmbeddingsInput::MultipleTokenIds(token_ids_list) => {
                if token_ids_list.is_empty() {
                    return Err("嵌入token ID列表不能为空".to_string());
                }
                if token_ids_list.len() > MAX_EMBEDDING_INPUTS {
                    return Err(format!("嵌入输入数量不能超过{}", MAX_EMBEDDING_INPUTS));
                }
                for (i, token_ids) in token_ids_list.iter().enumerate() {
                    if token_ids.is_empty() {
                        return Err(format!("嵌入输入{}的token ID列表不能为空", i));
                    }
                    if token_ids.len() > MAX_TEXT_LENGTH {
                        return Err(format!("嵌入输入{}的token ID数量过长", i));
                    }
                }
            }
        }
        Ok(())
    }

    /// 验证聊天完成请求的所有参数
    ///
    /// # 规则
    /// - 验证模型名称
    /// - 验证消息列表
    /// - 验证max_tokens
    /// - 验证temperature
    /// - 验证top_p
    /// - 验证频率和存在惩罚
    /// - 验证生成数量
    pub fn validate_chat_completion_request(
        params: &ChatCompletionRequestParams,
    ) -> ValidationResult<()> {
        Self::validate_model_name(params.model)?;
        Self::validate_messages(params.messages)?;
        Self::validate_max_tokens(params.max_tokens)?;
        Self::validate_temperature(params.temperature)?;
        Self::validate_top_p(params.top_p)?;
        Self::validate_frequency_penalty(params.frequency_penalty)?;
        Self::validate_presence_penalty(params.presence_penalty)?;
        Self::validate_n(params.n)?;
        Ok(())
    }

    /// 验证文本补全请求的所有参数
    ///
    /// # 规则
    /// - 验证模型名称
    /// - 验证prompt
    /// - 验证max_tokens
    /// - 验证temperature
    /// - 验证top_p
    /// - 验证频率和存在惩罚
    /// - 验证生成数量
    pub fn validate_completion_request(params: &CompletionRequestParams) -> ValidationResult<()> {
        Self::validate_model_name(params.model)?;
        Self::validate_prompt(params.prompt)?;
        Self::validate_max_tokens(params.max_tokens)?;
        Self::validate_temperature(params.temperature)?;
        Self::validate_top_p(params.top_p)?;
        Self::validate_frequency_penalty(params.frequency_penalty)?;
        Self::validate_presence_penalty(params.presence_penalty)?;
        Self::validate_n(params.n)?;
        Ok(())
    }

    /// 验证嵌入请求的所有参数
    ///
    /// # 规则
    /// - 验证模型名称
    /// - 验证输入
    pub fn validate_embeddings_request(
        model: &str,
        input: &super::embeddings::EmbeddingsInput,
    ) -> ValidationResult<()> {
        Self::validate_model_name(model)?;
        Self::validate_embedding_input(input)?;
        Ok(())
    }

    /// 安全范围调整 - 将数值调整到安全范围内
    ///
    /// # 说明
    /// - 如果值超出范围，自动调整到最近的边界值
    /// - 用于处理边界情况，避免验证失败
    pub fn safe_adjust_max_tokens(max_tokens: Option<u32>) -> Option<u32> {
        max_tokens.map(|tokens| tokens.clamp(MIN_TOKENS, MAX_TOKENS))
    }

    /// 安全范围调整 - 温度参数
    pub fn safe_adjust_temperature(temperature: Option<f32>) -> Option<f32> {
        temperature.map(|temp| temp.clamp(MIN_TEMPERATURE, MAX_TEMPERATURE))
    }

    /// 安全范围调整 - top_p参数
    pub fn safe_adjust_top_p(top_p: Option<f32>) -> Option<f32> {
        top_p.map(|p| p.clamp(MIN_TOP_P, MAX_TOP_P))
    }

    /// 安全范围调整 - 生成数量
    pub fn safe_adjust_n(n: Option<u32>) -> Option<u32> {
        n.map(|count| count.clamp(MIN_N, MAX_N))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dto::content::{ChatCompletionMessage, ChatMessageContent};

    #[test]
    fn test_validate_model_name() {
        assert!(Validator::validate_model_name("gpt-4").is_ok());
        assert!(Validator::validate_model_name("gpt-3.5-turbo").is_ok());
        assert!(Validator::validate_model_name("claude-2").is_ok());
        assert!(Validator::validate_model_name("openai/gpt-4").is_ok());
        assert!(Validator::validate_model_name("anthropic/claude-3").is_ok());
        assert!(Validator::validate_model_name("").is_err());
        assert!(Validator::validate_model_name("   ").is_err());
        assert!(Validator::validate_model_name("gpt@4").is_err());
    }

    #[test]
    fn test_validate_messages() {
        let valid_message = ChatCompletionMessage {
            role: "user".to_string(),
            content: Some(ChatMessageContent::String("Hello".to_string())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
            reasoning_content: None,
            extra_fields: serde_json::Value::default(),
        };

        assert!(Validator::validate_messages(&[valid_message.clone()]).is_ok());
        assert!(Validator::validate_messages(&[]).is_err());

        let empty_content = ChatCompletionMessage {
            role: "user".to_string(),
            content: Some(ChatMessageContent::String("   ".to_string())),
            name: None,
            tool_calls: None,
            tool_call_id: None,
            refusal: None,
            reasoning_content: None,
            extra_fields: serde_json::Value::default(),
        };
        assert!(Validator::validate_messages(&[empty_content]).is_err());
    }

    #[test]
    fn test_validate_max_tokens() {
        assert!(Validator::validate_max_tokens(Some(100)).is_ok());
        assert!(Validator::validate_max_tokens(Some(4096)).is_ok());
        assert!(Validator::validate_max_tokens(Some(0)).is_err());
        assert!(Validator::validate_max_tokens(Some(5000)).is_err());
    }

    #[test]
    fn test_validate_temperature() {
        assert!(Validator::validate_temperature(Some(0.5)).is_ok());
        assert!(Validator::validate_temperature(Some(0.0)).is_ok());
        assert!(Validator::validate_temperature(Some(2.0)).is_ok());
        assert!(Validator::validate_temperature(Some(-0.1)).is_err());
        assert!(Validator::validate_temperature(Some(2.1)).is_err());
    }

    #[test]
    fn test_safe_adjustments() {
        assert_eq!(Validator::safe_adjust_max_tokens(Some(0)), Some(1));
        assert_eq!(Validator::safe_adjust_max_tokens(Some(5000)), Some(4096));
        assert_eq!(Validator::safe_adjust_temperature(Some(-0.1)), Some(0.0));
        assert_eq!(Validator::safe_adjust_temperature(Some(2.5)), Some(2.0));
    }
}
