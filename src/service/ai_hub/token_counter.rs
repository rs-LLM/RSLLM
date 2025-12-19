//! Token计数服务模块
//! 
//! 提供多模态内容（文本、图像、音频）的token计数功能
//! 支持OpenAI模型的精确计算和非OpenAI模型的智能估算

use base64::{Engine, engine::general_purpose};
use image::ImageReader;
use std::io::Cursor;

/// Token计数元数据结构
#[derive(Debug, Clone, Default)]
pub struct TokenCountMeta {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub total_tokens: i64,
}

impl TokenCountMeta {
    pub fn new(input: i64, output: i64) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
        }
    }
    
    pub fn from_input(input: i64) -> Self {
        Self::new(input, 0)
    }
}

/// 多模态内容枚举
#[derive(Debug, Clone)]
pub enum Content {
    Text(String),
    Image(String), // base64编码
    Audio(String), // base64编码
    Multi(Vec<Content>),
}

/// 音频格式枚举
#[derive(Debug, Clone, PartialEq)]
pub enum AudioFormat {
    Wav,
    Mp3,
    M4a,
    Flac,
    Unknown,
}

impl From<&str> for AudioFormat {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "wav" => AudioFormat::Wav,
            "mp3" => AudioFormat::Mp3,
            "m4a" => AudioFormat::M4a,
            "flac" => AudioFormat::Flac,
            _ => AudioFormat::Unknown,
        }
    }
}

/// Token计数服务结构体
pub struct TokenCounter;

impl TokenCounter {
    /// 计算文本token数量
    /// 
    /// # Arguments
    /// * `text` - 文本内容
    /// * `model` - 模型名称
    /// 
    /// # Returns
    /// Token计数元数据
    /// 
    /// # Examples
    /// ```
    /// use rsllm::service::TokenCounter;
    /// // Returns Result<TokenCountMeta, String>
    /// let meta = TokenCounter::count_text_token("Hello world", "gpt-4");
    /// assert!(meta.is_ok());
    /// ```
    pub fn count_text_token(text: &str, model: &str) -> Result<TokenCountMeta, String> {
        if text.is_empty() {
            return Ok(TokenCountMeta::default());
        }

        // 对于所有模型，使用智能估算
        // 在实际项目中，这里应该集成tiktoken-rs或其他tokenizer库
        if Self::is_openai_model(model) {
            // OpenAI模型使用更精确的估算
            Ok(TokenCountMeta::from_input(Self::estimate_text_token_openai(text)))
        } else {
            // 非OpenAI模型使用通用估算
            Ok(TokenCountMeta::from_input(Self::estimate_text_token_by_model(model, text)))
        }
    }

    /// 计算图像token数量
    /// 
    /// # Arguments
    /// * `image_data` - base64编码的图像数据
    /// * `model` - 模型名称
    /// * `stream` - 是否为流式响应
    /// 
    /// # Returns
    /// Token计数元数据
    pub fn count_image_token(
        image_data: &str, 
        model: &str, 
        stream: bool
    ) -> Result<TokenCountMeta, String> {
        if image_data.is_empty() {
            return Ok(TokenCountMeta::default());
        }

        // 解码base64图像数据
        let image_bytes = general_purpose::STANDARD.decode(image_data)
            .map_err(|e| format!("Base64解码失败: {}", e))?;

        // 解析图像获取尺寸
        let image = ImageReader::new(Cursor::new(image_bytes))
            .with_guessed_format()
            .map_err(|e| format!("图像格式识别失败: {}", e))?
            .decode()
            .map_err(|e| format!("图像解码失败: {}", e))?;

        let width = image.width();
        let height = image.height();

        if width == 0 || height == 0 {
            return Ok(TokenCountMeta::from_input(85)); // 默认值
        }

        // 根据模型类型计算token
        let tokens = if Self::is_openai_model(model) {
            Self::calculate_openai_image_token(width, height, model, stream)
        } else {
            // 非OpenAI模型估算
            Self::estimate_image_token(width, height, model)
        };

        Ok(TokenCountMeta::from_input(tokens))
    }

    /// 计算音频token数量（按时长）
    /// 
    /// # Arguments
    /// * `audio_data` - base64编码的音频数据
    /// 
    /// # Returns
    /// Token计数元数据
    pub fn count_audio_token(audio_data: &str) -> Result<TokenCountMeta, String> {
        if audio_data.is_empty() {
            return Ok(TokenCountMeta::default());
        }

        // 解码base64音频数据
        let audio_bytes = general_purpose::STANDARD.decode(audio_data)
            .map_err(|e| format!("Base64解码失败: {}", e))?;

        // 估算音频时长（这里简化处理，实际应该解析音频头信息）
        // 假设音频数据大小与时间成正比，这里使用简化算法
        // 参考new-api: 1分钟 = 1000 token
        let duration_seconds = Self::estimate_audio_duration(&audio_bytes);
        let tokens = (duration_seconds / 60.0 * 1000.0).ceil() as i64;

        Ok(TokenCountMeta::from_input(tokens))
    }

    /// 统一处理多模态内容
    /// 
    /// # Arguments
    /// * `content` - 多模态内容
    /// * `model` - 模型名称
    /// * `stream` - 是否为流式响应
    /// 
    /// # Returns
    /// Token计数元数据
    pub fn count_content_tokens(
        content: &Content,
        model: &str,
        stream: bool,
    ) -> Result<TokenCountMeta, String> {
        match content {
            Content::Text(text) => Self::count_text_token(text, model),
            Content::Image(base64_data) => Self::count_image_token(base64_data, model, stream),
            Content::Audio(base64_data) => Self::count_audio_token(base64_data),
            Content::Multi(contents) => {
                let mut total_meta = TokenCountMeta::default();
                for content in contents {
                    let meta = Self::count_content_tokens(content, model, stream)?;
                    total_meta.input_tokens += meta.input_tokens;
                    total_meta.output_tokens += meta.output_tokens;
                    total_meta.total_tokens += meta.total_tokens;
                }
                Ok(total_meta)
            }
        }
    }

    /// 获取流式token统计（用于实时统计）
    /// 
    /// # Returns
    /// 流式token数量（这里返回0，实际使用时需要维护状态）
    pub fn get_stream_token_count() -> i64 {
        // 在实际应用中，这里应该返回累积的流式token数量
        // 由于是无状态的，这里返回0，需要外部维护状态
        0
    }

    // === 私有辅助方法 ===

    /// 检查是否为OpenAI模型
    fn is_openai_model(model: &str) -> bool {
        let lower = model.to_lowercase();
        lower.starts_with("gpt-") || 
        lower.starts_with("o1") || 
        lower.starts_with("o3") ||
        lower.starts_with("o4") ||
        lower.starts_with("chatgpt-") ||
        lower.starts_with("text-embedding-")
    }

    /// OpenAI模型的文本token估算（更精确）
    fn estimate_text_token_openai(text: &str) -> i64 {
        if text.is_empty() {
            return 0;
        }
        
        // 简化的OpenAI token估算规则
        // 实际应该使用tiktoken库，这里使用近似估算
        let chars: Vec<char> = text.chars().collect();
        let mut tokens = 0;
        
        for &c in &chars {
            if c.is_ascii() {
                // ASCII字符
                if c.is_alphanumeric() {
                    tokens += 1;
                } else if c.is_whitespace() {
                    tokens += 1;
                } else {
                    tokens += 1; // 标点符号
                }
            } else {
                // 非ASCII字符（中文等）
                tokens += 2; // 通常中文字符占2-3个token
            }
        }
        
        // 添加一些开销用于特殊字符和格式
        (tokens as f64 * 1.1).ceil() as i64
    }

    /// 通用文本token估算（字符数 × 系数）
    fn estimate_text_token(text: &str) -> i64 {
        // 中文字符通常占更多token，这里使用简单估算
        let chars = text.chars().count();
        let ascii_chars = text.chars().filter(|c| c.is_ascii()).count();
        let non_ascii_chars = chars - ascii_chars;
        
        // ASCII字符约0.3 token，非ASCII字符约0.6 token
        ((ascii_chars as f64 * 0.3) + (non_ascii_chars as f64 * 0.6)).ceil() as i64
    }

    /// 根据特定模型估算文本token
    fn estimate_text_token_by_model(model: &str, text: &str) -> i64 {
        let lower = model.to_lowercase();
        let base_estimate = Self::estimate_text_token(text);
        
        // 不同模型有不同的估算系数
        if lower.contains("claude") {
            // Claude的token计算方式略有不同
            (base_estimate as f64 * 1.1).ceil() as i64
        } else if lower.contains("gemini") {
            // Gemini的估算
            (base_estimate as f64 * 0.9).ceil() as i64
        } else if lower.contains("qwen") || lower.contains("baichuan") {
            // 国产模型估算
            (base_estimate as f64 * 1.0).ceil() as i64
        } else {
            base_estimate
        }
    }

    /// OpenAI图像token计算
    fn calculate_openai_image_token(width: u32, height: u32, model: &str, _stream: bool) -> i64 {
        let lower = model.to_lowercase();
        
        // 检查是否为patch-based模型（4.1-mini, 4.1-nano, o4-mini, gpt-5-mini/nano）
        let is_patch_based = lower.contains("gpt-4.1-mini") || 
                           lower.contains("gpt-4.1-nano") ||
                           lower.starts_with("o4-mini") ||
                           lower.starts_with("gpt-5-mini") ||
                           lower.starts_with("gpt-5-nano");
        
        if is_patch_based {
            return Self::calculate_patch_based_token(width, height, &lower);
        }

        // 默认tile-based计算参数
        let (base_tokens, tile_tokens) = if lower.starts_with("gpt-4o-mini") {
            (2833, 5667)
        } else if lower.starts_with("gpt-5-chat-latest") || 
                 (lower.starts_with("gpt-5") && !lower.contains("mini") && !lower.contains("nano")) {
            (70, 140)
        } else if lower.starts_with("o1") || lower.starts_with("o3") || lower.starts_with("o1-pro") {
            (75, 150)
        } else if lower.contains("computer-use-preview") {
            (65, 129)
        } else if lower.contains("4.1") || lower.contains("4o") || lower.contains("4.5") {
            (85, 170)
        } else {
            (85, 170) // 默认值
        };

        // Tile-based计算
        Self::calculate_tile_based_token(width, height, base_tokens, tile_tokens)
    }

    /// Patch-based token计算（用于4.1-mini, 4.1-nano等模型）
    fn calculate_patch_based_token(width: u32, height: u32, model: &str) -> i64 {
        let multiplier = if model.contains("gpt-4.1-mini") {
            1.62
        } else if model.contains("gpt-4.1-nano") {
            2.46
        } else if model.starts_with("o4-mini") {
            1.72
        } else if model.starts_with("gpt-5-mini") {
            1.62
        } else if model.starts_with("gpt-5-nano") {
            2.46
        } else {
            1.0
        };

        // 32x32 patch-based计算
        let ceil_div = |a: u32, b: u32| (a + b - 1) / b;
        let raw_patches_w = ceil_div(width, 32);
        let raw_patches_h = ceil_div(height, 32);
        let raw_patches = raw_patches_w * raw_patches_h;

        if raw_patches > 1536 {
            // 需要缩放
            let area = (width * height) as f64;
            let r = f64::sqrt((32.0 * 32.0 * 1536.0) / area);
            let w_scaled = width as f64 * r;
            let h_scaled = height as f64 * r;
            
            // 调整以适应缩放后的完整patch数
            let adj_w = (w_scaled / 32.0).floor() / (w_scaled / 32.0);
            let adj_h = (h_scaled / 32.0).floor() / (h_scaled / 32.0);
            let adj = f64::min(adj_w, adj_h);
            
            let final_r = if adj.is_finite() && adj > 0.0 {
                r * adj
            } else {
                r
            };

            let final_w = width as f64 * final_r;
            let final_h = height as f64 * final_r;
            let patches_w = f64::ceil(final_w / 32.0);
            let patches_h = f64::ceil(final_h / 32.0);
            
            let mut image_tokens = (patches_w * patches_h) as i64;
            if image_tokens > 1536 {
                image_tokens = 1536;
            }
            
            (image_tokens as f64 * multiplier).round() as i64
        } else {
            (raw_patches as f64 * multiplier).round() as i64
        }
    }

    /// Tile-based token计算
    fn calculate_tile_based_token(width: u32, height: u32, base_tokens: i64, tile_tokens: i64) -> i64 {
        // Step 1: 适配到2048x2048正方形
        let max_side = f64::max(width as f64, height as f64);
        let fit_scale = if max_side > 2048.0 {
            max_side / 2048.0
        } else {
            1.0
        };
        let fit_w = f64::round((width as f64) / fit_scale) as i64;
        let fit_h = f64::round((height as f64) / fit_scale) as i64;

        // Step 2: 缩放使最短边恰好为768
        let min_side = f64::min(fit_w as f64, fit_h as f64);
        if min_side == 0.0 {
            return base_tokens;
        }
        let short_scale = 768.0 / min_side;
        let final_w = f64::round((fit_w as f64) * short_scale) as i64;
        let final_h = f64::round((fit_h as f64) * short_scale) as i64;

        // Step 3: 计算512px tiles
        let tiles_w = (final_w + 512 - 1) / 512;
        let tiles_h = (final_h + 512 - 1) / 512;
        let tiles = tiles_w * tiles_h;

        tiles * tile_tokens + base_tokens
    }

    /// 非OpenAI模型图像token估算
    fn estimate_image_token(width: u32, height: u32, model: &str) -> i64 {
        let lower = model.to_lowercase();
        let area = (width * height) as f64;
        
        if lower.contains("claude") {
            // Claude使用类似的计算方式
            (area / 750.0).ceil() as i64
        } else if lower.contains("gemini") {
            // Gemini估算
            (area / 1000.0).ceil() as i64
        } else if lower.contains("glm-4") {
            // GLM-4特殊处理
            1047
        } else {
            // 通用估算
            (area / 800.0).ceil() as i64
        }
    }

    /// 估算音频时长（秒）
    fn estimate_audio_duration(audio_bytes: &[u8]) -> f64 {
        // 简化估算：假设16kHz单声道PCM，每个样本2字节
        // 实际应该解析音频头信息
        let bytes_per_second = 16000.0 * 2.0; // 16kHz, 16bit
        let duration = audio_bytes.len() as f64 / bytes_per_second;
        
        // 限制最小和最大时长
        f64::max(1.0, f64::min(duration, 3600.0)) // 1秒到1小时
    }
}
