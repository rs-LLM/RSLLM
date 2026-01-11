//! Token计数服务模块
//!
//! 提供多模态内容（文本、图像、音频）的token计数功能
//! 支持OpenAI模型的精确计算和非OpenAI模型的智能估算

use crate::routers::model_router::ModelRoutingInfo;
use base64::{Engine, engine::general_purpose};
use image::ImageReader;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, OnceLock};
use tiktoken_rs::CoreBPE;

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

    pub fn from_output(output: i64) -> Self {
        Self::new(0, output)
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

/// Chat消息角色
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

/// Chat消息结构
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub name: Option<String>,
}

/// Tokenizer缓存（单例模式）
static ENCODING_CACHE: OnceLock<Mutex<HashMap<String, Arc<CoreBPE>>>> = OnceLock::new();

/// Token计数服务结构体
pub struct TokenCounter;

impl TokenCounter {
    /// 计算文本token数量（精确计算）
    ///
    /// # 参数
    /// * `text` - 文本内容
    /// * `model` - 模型名称
    /// * `is_output` - 是否为输出内容
    ///
    /// # 返回
    /// * `Ok(TokenCountMeta)` - Token计数元数据
    /// * `Err(String)` - 错误信息
    ///
    /// # 示例
    /// ```
    /// use rsllm::service::ai_hub::token_counter::TokenCounter;
    /// let meta = TokenCounter::count_text_token("Hello world", "gpt-4", false)?;
    /// assert!(meta.input_tokens > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn count_text_token(
        text: &str,
        model: &str,
        is_output: bool,
    ) -> Result<TokenCountMeta, String> {
        log::info!(
            "[TokenCounter] count_text_token called: model={}, is_output={}, text_length={}",
            model,
            is_output,
            text.len()
        );

        if text.is_empty() {
            log::warn!("[TokenCounter] Text is empty, returning 0 tokens");
            return Ok(TokenCountMeta::default());
        }

        log::info!("[TokenCounter] Using o200k_base token counting for all models");
        let token_count = Self::count_text_token_openai(text, model)?;

        log::info!(
            "[TokenCounter] Token count result: token_count={}, is_output={}",
            token_count,
            is_output
        );

        if is_output {
            Ok(TokenCountMeta::from_output(token_count))
        } else {
            Ok(TokenCountMeta::from_input(token_count))
        }
    }

    /// 计算Chat消息列表的总token数
    ///
    /// # 参数
    /// * `model` - 模型名称
    /// * `messages` - 消息列表
    ///
    /// # 返回
    /// * `Ok(i64)` - 总token数
    /// * `Err(String)` - 错误信息
    ///
    /// # 计算规则
    /// - 每条消息添加固定开销：4 tokens (role + content分隔符)
    /// - 每个name字段添加固定开销：-1 tokens (如果存在)
    /// - 最后添加回复token：3 tokens (assistant开始标记)
    pub fn count_chat_tokens(model: &str, messages: &[ChatMessage]) -> Result<i64, String> {
        if messages.is_empty() {
            return Ok(3); // 只有回复token
        }

        let encoding_name = Self::get_encoding_for_model(model);
        let tokenizer = Self::get_cached_encoding(encoding_name)?;

        let mut total_tokens = 0;

        for message in messages {
            // 每条消息的基础开销：4 tokens
            total_tokens += 4;

            // 计算role的token数
            let role_str = match message.role {
                ChatRole::System => "system",
                ChatRole::User => "user",
                ChatRole::Assistant => "assistant",
                ChatRole::Tool => "tool",
            };
            let role_tokens = tokenizer.encode_with_special_tokens(role_str);
            total_tokens += role_tokens.len();

            // 计算content的token数
            if !message.content.is_empty() {
                let content_tokens = tokenizer.encode_with_special_tokens(&message.content);
                total_tokens += content_tokens.len();
            }

            // 如果有name字段，添加-1 tokens（因为已经计算了分隔符）
            if let Some(ref name) = message.name {
                let name_tokens = tokenizer.encode_with_special_tokens(name);
                total_tokens += name_tokens.len() - 1;
            }
        }

        // 添加回复token：3 tokens (assistant开始标记)
        total_tokens += 3;

        Ok(total_tokens as i64)
    }

    /// 计算图像token数量
    ///
    /// # 参数
    /// * `image_data` - base64编码的图像数据
    /// * `model` - 模型名称
    /// * `stream` - 是否为流式响应
    /// * `model_info` - 模型路由信息（可选，用于获取 token 计算参数）
    ///
    /// # 返回
    /// * `Ok(TokenCountMeta)` - Token计数元数据
    /// * `Err(String)` - 错误信息
    pub fn count_image_token(
        image_data: &str,
        model: &str,
        stream: bool,
        model_info: Option<&ModelRoutingInfo>,
    ) -> Result<TokenCountMeta, String> {
        if image_data.is_empty() {
            return Ok(TokenCountMeta::default());
        }

        let image_bytes = general_purpose::STANDARD
            .decode(image_data)
            .map_err(|e| format!("Base64解码失败: {}", e))?;

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

        log::info!("[TokenCounter] Using OpenAI image token calculation for all models");
        let tokens = Self::calculate_openai_image_token(width, height, model, stream, model_info);

        if stream {
            Ok(TokenCountMeta::from_output(tokens))
        } else {
            Ok(TokenCountMeta::from_input(tokens))
        }
    }

    /// 计算音频token数量（按时长）
    ///
    /// # 参数
    /// * `audio_data` - base64编码的音频数据
    /// * `model` - 模型名称
    /// * `stream` - 是否为流式响应
    /// * `model_info` - 模型路由信息（可选，用于获取 audio_tokens_per_second 参数）
    ///
    /// # 返回
    /// * `Ok(TokenCountMeta)` - Token计数元数据
    /// * `Err(String)` - 错误信息
    pub fn count_audio_token(
        audio_data: &str,
        _model: &str,
        stream: bool,
        model_info: Option<&ModelRoutingInfo>,
    ) -> Result<TokenCountMeta, String> {
        if audio_data.is_empty() {
            return Ok(TokenCountMeta::default());
        }

        let audio_bytes = general_purpose::STANDARD
            .decode(audio_data)
            .map_err(|e| format!("Base64解码失败: {}", e))?;

        let duration_seconds = Self::estimate_audio_duration(&audio_bytes);

        // 从 ModelRoutingInfo 获取每秒token数，或使用默认值
        let tokens_per_second = if let Some(info) = model_info {
            info.model_base.audio_tokens_per_second.unwrap_or(24.0)
        } else {
            // 默认值：24 tokens/秒（OpenAI Whisper）
            24.0
        };

        let tokens = (duration_seconds * tokens_per_second).ceil() as i64;

        if stream {
            Ok(TokenCountMeta::from_output(tokens))
        } else {
            Ok(TokenCountMeta::from_input(tokens))
        }
    }

    /// 统一处理多模态内容
    ///
    /// # 参数
    /// * `content` - 多模态内容
    /// * `model` - 模型名称
    /// * `stream` - 是否为流式响应
    /// * `model_info` - 模型路由信息（可选，用于获取 token 计算参数）
    ///
    /// # 返回
    /// * `Ok(TokenCountMeta)` - Token计数元数据
    /// * `Err(String)` - 错误信息
    pub fn count_content_tokens(
        content: &Content,
        model: &str,
        stream: bool,
        model_info: Option<&ModelRoutingInfo>,
    ) -> Result<TokenCountMeta, String> {
        match content {
            Content::Text(text) => Self::count_text_token(text, model, stream),
            Content::Image(base64_data) => {
                Self::count_image_token(base64_data, model, stream, model_info)
            }
            Content::Audio(base64_data) => {
                Self::count_audio_token(base64_data, model, stream, model_info)
            }
            Content::Multi(contents) => {
                let mut total_meta = TokenCountMeta::default();
                for content in contents {
                    let meta = Self::count_content_tokens(content, model, stream, model_info)?;
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
    /// # 返回
    /// 流式token数量（这里返回0，实际使用时需要维护状态）
    pub fn get_stream_token_count() -> i64 {
        0
    }

    // === 私有辅助方法 ===

    /// 获取模型对应的encoding名称
    ///
    /// # 参数
    /// * `model` - 模型名称（格式："provider_code/model_code" 或 "model_code"）
    ///
    /// # 返回
    /// * `&str` - 模型名称（用于 tiktoken_rs::get_bpe_from_model）
    fn get_encoding_for_model(model: &str) -> &'static str {
        log::info!("[TokenCounter] Getting encoding for model: {}", model);

        // 提取模型代号（去掉供应商代号部分）
        let model_code = if model.contains('/') {
            model.split('/').last().unwrap_or(model)
        } else {
            model
        };

        let lower = model_code.to_lowercase();

        log::info!("[TokenCounter] Extracted model code: {}", model_code);

        // o200k_base编码的模型（2024-2026新模型）
        // 使用 gpt-4o 作为代表模型来获取 o200k_base 编码
        if lower.starts_with("gpt-4o")
            || lower.starts_with("o1")
            || lower.starts_with("o3")
            || lower.starts_with("o4")
            || lower.starts_with("gpt-4.1")
            || lower.starts_with("gpt-5")
            || lower.starts_with("gpt-4.1-mini")
            || lower.starts_with("gpt-4.1-nano")
            || lower.starts_with("gpt-5-mini")
            || lower.starts_with("gpt-5-nano")
            || lower.starts_with("v3")
            || lower.starts_with("k")
            || lower.starts_with("1")
            || lower.starts_with("glm")
            || lower.starts_with("kimi")
            || lower.starts_with("deepseek")
        {
            log::info!(
                "[TokenCounter] Using o200k_base encoding (via gpt-4o) for model: {}",
                model_code
            );
            "gpt-4o"
        } else {
            // cl100k_base编码的模型（旧模型）
            log::info!(
                "[TokenCounter] Using cl100k_base encoding (via gpt-4) for model: {}",
                model_code
            );
            "gpt-4"
        }
    }

    /// 获取缓存的tokenizer实例
    ///
    /// # 参数
    /// * `encoding_name` - encoding名称，如"cl100k_base"或"o200k_base"
    ///
    /// # 返回
    /// * `Ok(Arc<CoreBPE>)` - 成功获取tokenizer实例
    /// * `Err(String)` - 加载失败时的错误信息
    ///
    /// # 性能说明
    /// - 首次调用会初始化tokenizer（约1-2ms）
    /// - 后续调用直接返回缓存实例（<1μs）
    /// - 使用OnceLock确保线程安全，无需额外锁
    fn get_cached_encoding(encoding_name: &str) -> Result<Arc<CoreBPE>, String> {
        let cache = ENCODING_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        {
            let mut cache_guard = cache.lock().expect("Failed to acquire cache lock");
            if !cache_guard.contains_key(encoding_name) {
                let bpe = tiktoken_rs::get_bpe_from_model(encoding_name)
                    .map_err(|e| format!("Failed to load encoding '{}': {}", encoding_name, e))?;
                cache_guard.insert(encoding_name.to_string(), Arc::new(bpe));
            }
        }

        let cache_guard = cache.lock().expect("Failed to acquire cache lock");
        cache_guard
            .get(encoding_name)
            .cloned()
            .ok_or_else(|| format!("Encoding '{}' not found in cache", encoding_name))
    }

    /// OpenAI模型的文本token计算（精确）
    ///
    /// # 参数
    /// * `text` - 文本内容
    /// * `model` - 模型名称
    ///
    /// # 返回
    /// * `Ok(i64)` - token数量
    /// * `Err(String)` - 错误信息
    fn count_text_token_openai(text: &str, model: &str) -> Result<i64, String> {
        if text.is_empty() {
            return Ok(0);
        }

        let encoding_name = Self::get_encoding_for_model(model);
        let tokenizer = Self::get_cached_encoding(encoding_name)?;

        let tokens = tokenizer.encode_with_special_tokens(text);

        Ok(tokens.len() as i64)
    }

    /// OpenAI图像token计算
    ///
    /// # 参数
    /// * `width` - 图像宽度
    /// * `height` - 图像高度
    /// * `model` - 模型名称
    /// * `stream` - 是否为流式响应
    /// * `model_info` - 模型路由信息（可选）
    ///
    /// # 返回
    /// token数量
    fn calculate_openai_image_token(
        width: u32,
        height: u32,
        model: &str,
        _stream: bool,
        model_info: Option<&ModelRoutingInfo>,
    ) -> i64 {
        let lower = model.to_lowercase();

        // 检查是否为patch-based模型（4.1-mini, 4.1-nano, o4-mini, gpt-5-mini/nano）
        let is_patch_based = lower.contains("gpt-4.1-mini")
            || lower.contains("gpt-4.1-nano")
            || lower.starts_with("o4-mini")
            || lower.starts_with("gpt-5-mini")
            || lower.starts_with("gpt-5-nano");

        if is_patch_based {
            return Self::calculate_patch_based_token(width, height, &lower, model_info);
        }

        // 从 ModelRoutingInfo 获取 tile-based 参数，或使用默认值
        let (base_tokens, tile_tokens) = if let Some(info) = model_info {
            let base = info.model_base.tile_base_tokens.unwrap_or(85);
            let tile = info.model_base.tile_tokens_per_tile.unwrap_or(170);
            (base, tile)
        } else {
            // 默认tile-based计算参数（已移除过时的gpt-4o-mini参数）
            if lower.starts_with("gpt-5-chat-latest")
                || (lower.starts_with("gpt-5")
                    && !lower.contains("mini")
                    && !lower.contains("nano"))
            {
                (70, 140)
            } else if lower.starts_with("o1")
                || lower.starts_with("o3")
                || lower.starts_with("o1-pro")
            {
                (75, 150)
            } else if lower.contains("computer-use-preview") {
                (65, 129)
            } else {
                (85, 170)
            }
        };

        Self::calculate_tile_based_token(width, height, base_tokens, tile_tokens)
    }

    /// Patch-based token计算（用于4.1-mini, 4.1-nano等模型）
    ///
    /// # 参数
    /// * `width` - 图像宽度
    /// * `height` - 图像高度
    /// * `model` - 模型名称（小写）
    /// * `model_info` - 模型路由信息（可选）
    ///
    /// # 返回
    /// token数量
    fn calculate_patch_based_token(
        width: u32,
        height: u32,
        model: &str,
        model_info: Option<&ModelRoutingInfo>,
    ) -> i64 {
        // 从 ModelRoutingInfo 获取乘数，或使用默认值
        let multiplier = if let Some(info) = model_info {
            info.model_base.patch_multiplier.unwrap_or(1.0)
        } else {
            // 默认乘数
            if model.contains("gpt-4.1-mini") {
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
            }
        };

        // 32x32 patch-based计算
        let ceil_div = |a: u32, b: u32| a.div_ceil(b);
        let raw_patches_w = ceil_div(width, 32);
        let raw_patches_h = ceil_div(height, 32);
        let raw_patches = raw_patches_w * raw_patches_h;

        if raw_patches > 1536 {
            // 需要缩放
            let area = (width * height) as f64;

            // 边界条件检查：防止除以零
            if area == 0.0 {
                return ((1536 as f64) * multiplier).round() as i64;
            }

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
    ///
    /// # 参数
    /// * `width` - 图像宽度
    /// * `height` - 图像高度
    /// * `base_tokens` - 基础token数
    /// * `tile_tokens` - 每个tile的token数
    ///
    /// # 返回
    /// token数量
    fn calculate_tile_based_token(
        width: u32,
        height: u32,
        base_tokens: i64,
        tile_tokens: i64,
    ) -> i64 {
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

        // 边界条件检查：防止除以零
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

    /// 估算音频时长（秒）
    ///
    /// # 参数
    /// * `audio_bytes` - 音频字节数据
    ///
    /// # 返回
    /// 音频时长（秒）
    ///
    /// # 说明
    /// - 简化估算：假设16kHz单声道PCM，每个样本2字节
    /// - 实际应该解析音频头信息以获得精确时长
    /// 估算音频时长（基于文件头识别格式）
    ///
    /// # 参数
    /// * `audio_bytes` - 音频字节数据
    ///
    /// # 返回
    /// 时长（秒）
    ///
    /// # 支持的格式
    /// - WAV: 16kHz, 16bit, 单声道
    /// - MP3: 128kbps, 估算
    /// - M4A/AAC: 128kbps, 估算
    /// - FLAC: 无损, 估算
    /// - 默认: 16kHz, 16bit
    fn estimate_audio_duration(audio_bytes: &[u8]) -> f64 {
        if audio_bytes.len() < 4 {
            return 1.0; // 最小时长
        }

        let (_format, bytes_per_second) = Self::detect_audio_format(audio_bytes);

        let duration = audio_bytes.len() as f64 / bytes_per_second;
        duration.clamp(1.0, 3600.0) // 1秒到1小时
    }

    /// 检测音频格式并返回对应的字节率
    ///
    /// # 参数
    /// * `audio_bytes` - 音频字节数据
    ///
    /// # 返回
    /// * `(format, bytes_per_second)` - 格式名称和每秒字节数
    fn detect_audio_format(audio_bytes: &[u8]) -> (&'static str, f64) {
        let header = &audio_bytes[..audio_bytes.len().min(12)];

        // WAV 格式检测 (RIFF header)
        if header.len() >= 12 && &header[0..4] == b"RIFF" && &header[8..12] == b"WAVE" {
            return ("WAV", 16000.0 * 2.0); // 16kHz, 16bit
        }

        // MP3 格式检测 (ID3v2 或 MP3 frame sync)
        if header.len() >= 3 && &header[0..3] == b"ID3" {
            return ("MP3", 128000.0 / 8.0); // 128kbps
        }
        if header.len() >= 2 && (header[0] == 0xFF && (header[1] & 0xE0) == 0xE0) {
            return ("MP3", 128000.0 / 8.0); // 128kbps
        }

        // M4A/AAC 格式检测 (ftyp box)
        if header.len() >= 8 && &header[4..8] == b"ftyp" {
            return ("M4A", 128000.0 / 8.0); // 128kbps
        }

        // FLAC 格式检测 (fLaC marker)
        if header.len() >= 4 && &header[0..4] == b"fLaC" {
            return ("FLAC", 500000.0 / 8.0); // 估算 500kbps
        }

        // 默认使用 16kHz, 16bit
        ("Unknown", 16000.0 * 2.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let meta = TokenCounter::count_text_token("", "gpt-4", false)
            .expect("Failed to count empty text tokens");
        assert_eq!(meta.input_tokens, 0);
    }

    #[test]
    fn test_chat_tokens_empty() {
        let messages: Vec<ChatMessage> = vec![];
        let tokens = TokenCounter::count_chat_tokens("gpt-4", &messages)
            .expect("Failed to count chat tokens for empty messages");
        assert_eq!(tokens, 3); // 只有回复token
    }

    #[test]
    fn test_chat_tokens_single_message() {
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: "Hello".to_string(),
            name: None,
        }];
        let tokens = TokenCounter::count_chat_tokens("gpt-4", &messages)
            .expect("Failed to count chat tokens for single message");
        assert!(tokens > 0);
    }

    #[test]
    fn test_text_token_calculation() {
        let meta = TokenCounter::count_text_token("Hello, world!", "gpt-4", false)
            .expect("Failed to count text tokens");
        assert!(meta.input_tokens > 0);
        assert_eq!(meta.output_tokens, 0);
        assert_eq!(meta.total_tokens, meta.input_tokens);
    }

    #[test]
    fn test_text_token_with_stream() {
        let meta = TokenCounter::count_text_token("Hello, world!", "gpt-4", true)
            .expect("Failed to count text tokens with stream");
        assert!(meta.output_tokens > 0);
        assert_eq!(meta.input_tokens, 0);
        assert_eq!(meta.total_tokens, meta.output_tokens);
    }

    #[test]
    fn test_text_token_multilingual() {
        let chinese = "你好，世界！";
        let meta = TokenCounter::count_text_token(chinese, "gpt-4", false)
            .expect("Failed to count Chinese text tokens");
        assert!(meta.input_tokens > 0);
    }

    #[test]
    fn test_audio_format_detection() {
        let wav_header = b"RIFF\x24\x00\x00\x00WAVE";
        let (format, bytes_per_sec) = TokenCounter::detect_audio_format(wav_header);
        assert_eq!(format, "WAV");
        assert_eq!(bytes_per_sec, 32000.0);

        let mp3_header = b"ID3\x04\x00\x00\x00\x00\x00\x00";
        let (format, bytes_per_sec) = TokenCounter::detect_audio_format(mp3_header);
        assert_eq!(format, "MP3");
        assert_eq!(bytes_per_sec, 16000.0);

        let m4a_header = b"\x00\x00\x00\x20ftyp";
        let (format, bytes_per_sec) = TokenCounter::detect_audio_format(m4a_header);
        assert_eq!(format, "M4A");
        assert_eq!(bytes_per_sec, 16000.0);

        let flac_header = b"fLaC";
        let (format, bytes_per_sec) = TokenCounter::detect_audio_format(flac_header);
        assert_eq!(format, "FLAC");
        assert_eq!(bytes_per_sec, 62500.0);
    }

    #[test]
    fn test_audio_duration_estimation() {
        let wav_header = b"RIFF\x24\x00\x00\x00WAVE";
        let audio_data = [wav_header.to_vec(), vec![0u8; 32000]].concat();
        let duration = TokenCounter::estimate_audio_duration(&audio_data);
        assert!(duration >= 1.0);
    }

    #[test]
    fn test_audio_token_calculation() {
        let wav_header = b"RIFF\x24\x00\x00\x00WAVE";
        let audio_data = [wav_header.to_vec(), vec![0u8; 32000]].concat();
        let audio_base64 = general_purpose::STANDARD.encode(&audio_data);

        let meta = TokenCounter::count_audio_token(&audio_base64, "gpt-4", false, None)
            .expect("Failed to count audio tokens");
        assert!(meta.input_tokens > 0);
    }
}
