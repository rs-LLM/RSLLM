use std::time::Duration;

// 用途：导入全局上下文
// 说明：用于访问缓存服务和配置信息
use crate::context::CONTEXT;

// 用途：导入验证码DTO
// 说明：用于接收验证码请求参数
use crate::domain::dto::CatpchaDTO;

// 用途：导入错误信息宏
// 说明：用于生成错误信息
use crate::error_info;

// 用途：导入字符串扩展特性
// 说明：用于检查字符串是否为空
use crate::util::string::IsEmptyString;

// 用途：导入axum的Body类型
// 说明：用于构建HTTP响应体
use axum::body::Body;

// 用途：导入axum的Query提取器
// 说明：用于从URL查询参数中提取数据
use axum::extract::Query;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::{IntoResponse, Response};

// 用途：导入验证码生成库
// 说明：用于生成验证码图片和字符串
use captcha::Captcha;

// 用途：导入验证码过滤器
// 说明：用于添加验证码图片的干扰效果
use captcha::filters::{Dots, Noise, Wave};

/// 用途：验证码接口
/// 说明：生成和返回验证码图片，用于用户登录验证
/// Http Method GET
/// example：
/// http://localhost:8000/admin/captcha?account=18900000000
#[utoipa::path(
    get,
    path = "/admin/captcha",
    params(
        ("account" = Option<String>, Query, description = "账号")
    ),
    responses(
        (status = 200, description = "验证码图片", content_type = "image/png"),
        (status = 400, description = "账号为空", body = String)
    ),
    tag = "captcha"
)]
pub async fn captcha(arg: Query<CatpchaDTO>) -> impl IntoResponse {
    // 用途：检查账号是否为空
    // 说明：验证码需要与账号关联，不能为空
    if arg.account.is_empty() {
        // 用途：构建错误响应
        // 说明：当账号为空时返回错误信息
        let resp = Response::builder()
            .header("Access-Control-Allow-Origin", "*") // 用途：允许跨域请求
            // 说明：验证码不需要缓存
            .header("Cache-Control", "no-cache")
            .header("Content-Type", "json") // 用途：设置响应类型为JSON
            // 说明：返回错误信息
            .body(Body::from(error_info!("account_empty")))
            .unwrap_or_default();
        return resp;
    }
    // 用途：生成验证码
    // 说明：调用make函数生成验证码图片和字符串
    let (png, code) = make();
    // 用途：检查是否为调试模式
    // 说明：调试模式下输出验证码日志，方便调试
    // 注意：不要打印真实验证码，避免泄露
    if CONTEXT.config.debug() {
        // 用途：输出验证码日志
        // 说明：记录账号，便于调试
        log::info!(
            "captcha_generated account:{}",
            arg.account.as_deref().unwrap_or_default()
        );
    }
    // 用途：检查账号是否存在
    // 说明：只有当账号存在时才将验证码存入缓存
    if arg.account.is_some() {
        // 用途：将验证码存入缓存
        // 说明：用于后续验证用户输入的验证码是否正确
        let result = CONTEXT
            .cache_service
            .set_string_ex(
                // 用途：构建缓存键
                // 说明：使用账号作为缓存键的一部分，确保每个账号的验证码唯一
                &format!(
                    "captch:account_{}",
                    arg.account.as_deref().unwrap_or_default()
                ),
                code.as_str(),
                Some(Duration::from_secs(300)),
            )
            .await;
        // 用途：检查是否为发布模式
        // 说明：发布模式下需要处理缓存错误
        if !CONTEXT.config.debug() {
            // 用途：处理缓存错误
            // 说明：发布模式下缓存错误需要返回给客户端
            if let Err(e) = result {
                // 用途：构建错误响应
                // 说明：当缓存操作失败时返回错误信息
                let resp = Response::builder()
                    .header("Access-Control-Allow-Origin", "*")
                    .header("Cache-Control", "no-cache")
                    .header("Content-Type", "json")
                    .body(Body::from(e.to_string()))
                    .unwrap_or_default();
                return resp;
            }
        }
    }
    // 用途：构建验证码响应
    // 说明：返回生成的验证码图片
    Response::builder()
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-cache")
        .header("Content-Type", "image/png")
        .body(Body::from(png))
        .unwrap_or_default()
}

/// 用途：生成验证码图片和字符串
/// 说明：创建包含干扰效果的验证码
fn make() -> (Vec<u8>, String) {
    // 用途：创建验证码实例
    // 说明：用于生成验证码
    let mut captcha = Captcha::new();
    // 用途：生成4位验证码字符
    // 说明：设置验证码的长度
    captcha
        .add_chars(4)
        // 用途：添加噪声干扰
        // 说明：增加验证码的安全性，防止自动识别
        .apply_filter(Noise::new(0.1))
        // 用途：添加水平波浪干扰
        // 说明：增加验证码的安全性，防止自动识别
        .apply_filter(Wave::new(1.0, 10.0).horizontal())
        // 用途：注释掉的垂直波浪干扰
        // 说明：可以根据需要启用，增加验证码的复杂度
        // .apply_filter(Wave::new(2.0, 20.0).vertical())
        // 用途：设置验证码图片尺寸
        // 说明：定义验证码图片的宽高
        .view(160, 60)
        // 用途：添加点阵干扰
        // 说明：增加验证码的安全性，防止自动识别
        .apply_filter(Dots::new(4));
    // 用途：生成PNG图片
    // 说明：将验证码转换为图片格式
    let png = captcha.as_png().unwrap_or_default();
    // 用途：获取验证码字符串并转换为小写
    // 说明：统一验证码的大小写，方便后续验证
    let captcha_str = captcha.chars_as_string().to_lowercase();
    // 用途：返回验证码图片和字符串
    // 说明：将生成的验证码数据返回给调用者
    (png, captcha_str)
}
