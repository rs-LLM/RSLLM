// 用途：导入serde的序列化和反序列化特性
// 说明：支持AI相关枚举的JSON序列化和反序列化
use serde::{Deserializer, Serializer};
// 用途：导入格式化相关特性
// 说明：支持AI相关枚举的调试和显示格式化
use std::fmt::{Debug, Display, Formatter};

// 用途：AI模型类型枚举
// 说明：定义不同的AI模型类型，用于区分聊天、补全、嵌入等模型
#[derive(Clone, Debug)]
pub enum ModelType {
    // 用途：聊天模型
    // 说明：用于对话聊天的模型，如GPT-3.5、Claude等
    Chat,
    // 用途：文本补全模型
    // 说明：用于文本生成和补全的模型
    Completion,
    // 用途：文本嵌入模型
    // 说明：用于生成文本向量嵌入的模型
    Embedding,
    // 用途：图像生成模型
    // 说明：用于生成图像的模型
    ImageGeneration,
    // 用途：语音转文本模型
    // 说明：用于语音识别的模型
    SpeechToText,
    // 用途：文本转语音模型
    // 说明：用于文本转语音的模型
    TextToSpeech,
}

// 用途：为ModelType实现Default trait
// 说明：提供默认值，方便在未指定时使用
impl Default for ModelType {
    // 用途：默认值方法
    // 说明：默认为聊天模型
    fn default() -> Self {
        ModelType::Chat
    }
}

// 用途：实现ModelType到&str的转换
// 说明：方便将枚举转换为字符串，用于存储和传输
impl From<ModelType> for &str {
    // 用途：转换方法
    // 说明：将不同的模型类型映射为对应的字符串
    fn from(arg: ModelType) -> Self {
        match arg {
            ModelType::Chat => "chat",
            ModelType::Completion => "completion",
            ModelType::Embedding => "embedding",
            ModelType::ImageGeneration => "image_generation",
            ModelType::SpeechToText => "speech_to_text",
            ModelType::TextToSpeech => "text_to_speech",
        }
    }
}

// 用途：实现&str到ModelType的转换
// 说明：方便将字符串转换为枚举，用于从存储或传输中恢复
impl From<&str> for ModelType {
    // 用途：转换方法
    // 说明：将字符串映射为对应的模型类型，支持容错处理
    fn from(arg: &str) -> Self {
        match arg {
            "chat" => ModelType::Chat,
            "completion" => ModelType::Completion,
            "embedding" => ModelType::Embedding,
            "image_generation" => ModelType::ImageGeneration,
            "speech_to_text" => ModelType::SpeechToText,
            "text_to_speech" => ModelType::TextToSpeech,
            _ => ModelType::Chat,
        }
    }
}

// 用途：实现Display trait
// 说明：支持直接打印，便于日志记录和用户界面显示
impl Display for ModelType {
    // 用途：显示格式化方法
    // 说明：将枚举转换为字符串进行显示输出
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&str>::from(self.clone()))
    }
}

// 用途：实现Serialize trait
// 说明：支持JSON序列化，方便在网络中传输
impl serde::Serialize for ModelType {
    // 用途：序列化方法
    // 说明：将枚举转换为字符串后进行序列化
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

// 用途：实现Deserialize trait
// 说明：支持JSON反序列化，方便从网络中接收数据
impl<'de> serde::Deserialize<'de> for ModelType {
    // 用途：反序列化方法
    // 说明：将字符串反序列化为对应的枚举类型
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;
        Ok(ModelType::from(v.as_str()))
    }
}

// 用途：AI提供商类型枚举
// 说明：定义不同的AI服务提供商，用于区分不同的AI服务
#[derive(Clone, Debug)]
pub enum ProviderType {
    // 用途：OpenAI提供商
    // 说明：OpenAI公司提供的AI服务
    OpenAI,
    // 用途：Anthropic提供商
    // 说明：Anthropic公司提供的AI服务
    Anthropic,
    // 用途：Google提供商
    // 说明：Google公司提供的AI服务
    Google,
    // 用途：Microsoft提供商
    // 说明：Microsoft公司提供的AI服务
    Microsoft,
    // 用途：Kimi提供商
    // 说明：Kimi公司提供的AI服务
    Kimi,
    // 用途：Qwen提供商
    // 说明：阿里公司提供的AI服务
    Qwen,
    // 用途：Doubao提供商
    // 说明：Doubao公司提供的AI服务
    Doubao,
    // 用途：MiniMAX提供商
    // 说明：MiniMAX公司提供的AI服务
    MiniMAX,
    // 用途：SiliconFlow提供商
    // 说明：SiliconFlow公司提供的AI服务
    SiliconFlow,
    // 用途：自定义提供商
    // 说明：其他自定义的AI服务提供商
    Custom(String),
}

// 用途：为ProviderType实现Default trait
// 说明：提供默认值，方便在未指定时使用
impl Default for ProviderType {
    // 用途：默认值方法
    // 说明：默认为OpenAI
    fn default() -> Self {
        ProviderType::OpenAI
    }
}

// 用途：实现ProviderType到&str的转换
// 说明：方便将枚举转换为字符串，用于存储和传输
impl From<ProviderType> for &str {
    // 用途：转换方法
    // 说明：将不同的提供商类型映射为对应的字符串
    fn from(arg: ProviderType) -> Self {
        match arg {
            ProviderType::OpenAI => "openai",
            ProviderType::Anthropic => "anthropic",
            ProviderType::Google => "google",
            ProviderType::Microsoft => "microsoft",
            ProviderType::Kimi => "kimi",
            ProviderType::Qwen => "qwen",
            ProviderType::Doubao => "doubao",
            ProviderType::MiniMAX => "minimax",
            ProviderType::SiliconFlow => "siliconflow",
            ProviderType::Custom(_) => "custom",
        }
    }
}

// 用途：实现&str到ProviderType的转换
// 说明：方便将字符串转换为枚举，用于从存储或传输中恢复
impl From<&str> for ProviderType {
    // 用途：转换方法
    // 说明：将字符串映射为对应的提供商类型，支持容错处理
    fn from(arg: &str) -> Self {
        match arg {
            "openai" => ProviderType::OpenAI,
            "anthropic" => ProviderType::Anthropic,
            "google" => ProviderType::Google,
            "microsoft" => ProviderType::Microsoft,
            "kimi" => ProviderType::Kimi,
            "qwen" => ProviderType::Qwen,
            "doubao" => ProviderType::Doubao,
            "minimax" => ProviderType::MiniMAX,
            "siliconflow" => ProviderType::SiliconFlow,
            "custom" => ProviderType::Custom("custom".to_string()),
            _ => ProviderType::OpenAI,
        }
    }
}

// 用途：实现Display trait
// 说明：支持直接打印，便于日志记录和用户界面显示
impl Display for ProviderType {
    // 用途：显示格式化方法
    // 说明：将枚举转换为字符串进行显示输出
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&str>::from(self.clone()))
    }
}

// 用途：实现Serialize trait
// 说明：支持JSON序列化，方便在网络中传输
impl serde::Serialize for ProviderType {
    // 用途：序列化方法
    // 说明：将枚举转换为字符串后进行序列化
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

// 用途：实现Deserialize trait
// 说明：支持JSON反序列化，方便从网络中接收数据
impl<'de> serde::Deserialize<'de> for ProviderType {
    // 用途：反序列化方法
    // 说明：将字符串反序列化为对应的枚举类型
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;
        Ok(ProviderType::from(v.as_str()))
    }
}

// 用途：管道类型枚举
// 说明：定义不同的AI处理管道类型，用于区分不同的处理流程
#[derive(Clone, Debug)]
pub enum PipelineType {
    // 用途：聊天管道
    // 说明：用于处理聊天对话的管道
    Chat,
    // 用途：文本补全管道
    // 说明：用于处理文本生成和补全的管道
    Completion,
    // 用途：嵌入生成管道
    // 说明：用于生成文本向量嵌入的管道
    Embedding,
    // 用途：图像生成管道
    // 说明：用于生成图像的管道
    ImageGeneration,
    // 用途：语音处理管道
    // 说明：用于处理语音相关任务的管道
    Speech,
    // 用途：多模态管道
    // 说明：用于处理多种模态数据的管道
    Multimodal,
    // 用途：自定义管道
    // 说明：其他自定义的处理管道
    Custom(String),
}

// 用途：为PipelineType实现Default trait
// 说明：提供默认值，方便在未指定时使用
impl Default for PipelineType {
    // 用途：默认值方法
    // 说明：默认为聊天管道
    fn default() -> Self {
        PipelineType::Chat
    }
}

// 用途：实现PipelineType到&str的转换
// 说明：方便将枚举转换为字符串，用于存储和传输
impl From<PipelineType> for &str {
    // 用途：转换方法
    // 说明：将不同的管道类型映射为对应的字符串
    fn from(arg: PipelineType) -> Self {
        match arg {
            PipelineType::Chat => "chat",
            PipelineType::Completion => "completion",
            PipelineType::Embedding => "embedding",
            PipelineType::ImageGeneration => "image_generation",
            PipelineType::Speech => "speech",
            PipelineType::Multimodal => "multimodal",
            PipelineType::Custom(_) => "custom",
        }
    }
}

// 用途：实现&str到PipelineType的转换
// 说明：方便将字符串转换为枚举，用于从存储或传输中恢复
impl From<&str> for PipelineType {
    // 用途：转换方法
    // 说明：将字符串映射为对应的管道类型，支持容错处理
    fn from(arg: &str) -> Self {
        match arg {
            "chat" => PipelineType::Chat,
            "completion" => PipelineType::Completion,
            "embedding" => PipelineType::Embedding,
            "image_generation" => PipelineType::ImageGeneration,
            "speech" => PipelineType::Speech,
            "multimodal" => PipelineType::Multimodal,
            "custom" => PipelineType::Custom("custom".to_string()),
            _ => PipelineType::Chat,
        }
    }
}

// 用途：实现Display trait
// 说明：支持直接打印，便于日志记录和用户界面显示
impl Display for PipelineType {
    // 用途：显示格式化方法
    // 说明：将枚举转换为字符串进行显示输出
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(<&str>::from(self.clone()))
    }
}

// 用途：实现Serialize trait
// 说明：支持JSON序列化，方便在网络中传输
impl serde::Serialize for PipelineType {
    // 用途：序列化方法
    // 说明：将枚举转换为字符串后进行序列化
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

// 用途：实现Deserialize trait
// 说明：支持JSON反序列化，方便从网络中接收数据
impl<'de> serde::Deserialize<'de> for PipelineType {
    // 用途：反序列化方法
    // 说明：将字符串反序列化为对应的枚举类型
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = String::deserialize(deserializer)?;
        Ok(PipelineType::from(v.as_str()))
    }
}
