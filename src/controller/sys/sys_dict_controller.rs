// 用途：导入全局上下文
// 说明：用于访问系统字典服务
use crate::context::CONTEXT;

// 用途：导入字典相关的数据传输对象
// 说明：用于接收字典的添加、编辑、分页查询和删除请求参数
use crate::domain::dto::{DictAddDTO, DictEditDTO, DictPageDTO, IdDTO};

// 用途：导入字典表结构
// 说明：用于将DTO转换为数据库实体
use crate::domain::table::sys_dict::SysDict;

// 用途：导入响应VO
// 说明：用于统一HTTP响应格式
use crate::domain::vo::RespVO;

// 用途：导入错误信息宏
// 说明：用于生成错误信息
use crate::error_info;

// 用途：导入axum的Json提取器
// 说明：用于从HTTP请求体中提取JSON数据
use axum::Json;

// 用途：导入响应转换特性
// 说明：用于将函数返回值转换为HTTP响应
use axum::response::IntoResponse;

/// 用途：分页查询字典
/// 说明：处理字典的分页查询请求
pub async fn page(page: Json<DictPageDTO>) -> impl IntoResponse {
    // 用途：调用字典服务分页查询字典
    // 说明：从数据库中分页查询字典数据
    let data = CONTEXT.sys_dict_service.page(&page.0).await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：添加字典
/// 说明：处理字典的添加请求
pub async fn add(mut arg: Json<DictAddDTO>) -> impl IntoResponse {
    // 用途：检查字典名称是否为空
    // 说明：字典名称是必填项
    if arg.name.is_none() {
        return RespVO::<u64>::from_error(error_info!("empty"));
    }
    // 用途：检查字典编码是否为空
    // 说明：字典编码是必填项，用于唯一标识字典
    if arg.code.is_none() {
        return RespVO::<u64>::from_error(error_info!("empty"));
    }
    // 用途：检查字典状态是否为空
    // 说明：字典状态为空时，默认设置为启用状态
    if arg.state.is_none() {
        arg.state = Some(1); // 用途：默认启用状态
                           // 说明：字典创建时默认可用
    }
    // 用途：将DTO转换为数据库实体
    // 说明：数据库操作需要使用实体对象
    let res = SysDict::from(arg.0);
    // 用途：调用字典服务添加字典
    // 说明：将新字典数据保存到数据库
    let data = CONTEXT.sys_dict_service.add(&res).await;
    // 用途：更新字典缓存
    // 说明：字典数据发生变化时，需要更新缓存，确保后续查询使用最新数据
    let _ = CONTEXT.sys_dict_service.update_cache().await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：更新字典
/// 说明：处理字典的更新请求
pub async fn update(arg: Json<DictEditDTO>) -> impl IntoResponse {
    // 用途：调用字典服务更新字典
    // 说明：更新数据库中的字典数据
    let data = CONTEXT.sys_dict_service.edit(&arg.0).await;
    // 用途：更新字典缓存
    // 说明：字典数据发生变化时，需要更新缓存，确保后续查询使用最新数据
    let _ = CONTEXT.sys_dict_service.update_cache().await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}

/// 用途：删除字典
/// 说明：处理字典的删除请求
pub async fn remove(arg: Json<IdDTO>) -> impl IntoResponse {
    // 用途：调用字典服务删除字典
    // 说明：从数据库中删除指定ID的字典
    let data = CONTEXT
        .sys_dict_service
        .remove(&arg.0.id.unwrap_or_default())
        .await;
    // 用途：更新字典缓存
    // 说明：字典数据发生变化时，需要更新缓存，确保后续查询使用最新数据
    let _ = CONTEXT.sys_dict_service.update_cache().await;
    // 用途：将结果转换为响应VO
    // 说明：统一响应格式，包含状态码、消息和数据
    RespVO::from_result(data)
}
