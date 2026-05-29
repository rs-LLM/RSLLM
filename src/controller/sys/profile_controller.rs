use std::sync::Arc;

use axum::Json;
use axum::extract::{Multipart, Query, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::context::ServiceContext;
use crate::domain::table::sys_user::SysUser;
use crate::domain::vo::response::ApiResponse;
use crate::error::{ApplicationError, ApplicationResult};
use crate::middleware::auth_axum::JwtAuth;

#[derive(Debug, Deserialize)]
pub struct ReadFileQuery {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UploadFileResponse {
    pub path: String,
    pub url: String,
}

fn is_safe_path(path: &str) -> bool {
    !path.contains("..") && path.starts_with("uploads/")
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn content_type_by_ext(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else {
        "application/octet-stream"
    }
}

async fn save_upload(
    mut multipart: Multipart,
    prefix: &str,
    user_id: &str,
) -> ApplicationResult<(UploadFileResponse, Vec<u8>)> {
    let mut bytes: Option<Vec<u8>> = None;
    let mut filename = "file.bin".to_string();

    while let Some(field) =
        multipart
            .next_field()
            .await
            .map_err(|e| ApplicationError::ValidationError {
                message: format!("读取上传文件失败: {}", e),
                field: Some("file".to_string()),
                value: None,
            })?
    {
        if field.name() == Some("file") {
            filename = sanitize_filename(field.file_name().unwrap_or("file.bin"));
            bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| ApplicationError::ValidationError {
                        message: format!("读取文件字节失败: {}", e),
                        field: Some("file".to_string()),
                        value: None,
                    })?
                    .to_vec(),
            );
            break;
        }
    }

    let data = bytes.ok_or_else(|| ApplicationError::ValidationError {
        message: "请上传文件字段 file".to_string(),
        field: Some("file".to_string()),
        value: None,
    })?;

    if data.is_empty() {
        return Err(ApplicationError::ValidationError {
            message: "上传文件不能为空".to_string(),
            field: Some("file".to_string()),
            value: None,
        });
    }

    let ext = filename
        .rsplit('.')
        .next()
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "bin".to_string());
    let path = format!("uploads/{}/{}/{}.{}", prefix, user_id, Ulid::new(), ext);

    Ok((
        UploadFileResponse {
            path,
            url: String::new(),
        },
        data,
    ))
}

#[derive(Debug, Serialize)]
pub struct MyProfileResponse {
    pub id: Option<String>,
    pub account: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub user_level: Option<String>,
}

pub async fn read_public_file(
    State(context): State<Arc<ServiceContext>>,
    Query(query): Query<ReadFileQuery>,
) -> impl IntoResponse {
    if !is_safe_path(&query.path) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<serde_json::Value>::error(
                "INVALID_PATH",
                "非法文件路径",
            )),
        )
            .into_response();
    }

    match context.storage_service.download(query.path.clone()).await {
        Ok(bytes) => {
            let content_type = content_type_by_ext(&query.path);
            let mut response = (StatusCode::OK, bytes).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(content_type)
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            response
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<serde_json::Value>::error(
                "FILE_NOT_FOUND",
                &format!("文件读取失败: {}", e),
            )),
        )
            .into_response(),
    }
}

pub async fn upload_avatar(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    multipart: Multipart,
) -> ApplicationResult<Json<ApiResponse<UploadFileResponse>>> {
    let (mut upload, bytes) = save_upload(multipart, "avatars", &jwt_auth.id).await?;
    context
        .storage_service
        .upload(upload.path.clone(), bytes)
        .await?;
    upload.url = format!("/rsllm/public/file/read?path={}", upload.path);

    let mut users =
        SysUser::select_by_map(context.rb.as_ref(), rbs::value! {"id": jwt_auth.id.clone()})
            .await?;
    let mut user = users
        .drain(..)
        .next()
        .ok_or_else(|| ApplicationError::NotFound {
            message: "用户不存在".to_string(),
            resource: Some("sys_user".to_string()),
            id: Some(jwt_auth.id.clone()),
        })?;
    user.avatar = Some(upload.url.clone());
    SysUser::update_by_map(
        context.rb.as_ref(),
        &user,
        rbs::value! {"id": jwt_auth.id.clone()},
    )
    .await?;
    Ok(Json(ApiResponse::success(upload)))
}

pub async fn upload_image(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    multipart: Multipart,
) -> ApplicationResult<Json<ApiResponse<UploadFileResponse>>> {
    let (mut upload, data) = save_upload(multipart, "site", &jwt_auth.id).await?;
    context
        .storage_service
        .upload(upload.path.clone(), data)
        .await?;
    upload.url = format!("/rsllm/public/file/read?path={}", upload.path);
    Ok(Json(ApiResponse::success(upload)))
}

pub async fn get_profile(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
) -> ApplicationResult<Json<ApiResponse<SysUser>>> {
    let mut users =
        SysUser::select_by_map(context.rb.as_ref(), rbs::value! {"id": jwt_auth.id.clone()})
            .await?;
    let user = users
        .drain(..)
        .next()
        .ok_or_else(|| ApplicationError::NotFound {
            message: "用户不存在".to_string(),
            resource: Some("sys_user".to_string()),
            id: None,
        })?;
    Ok(Json(ApiResponse::success(SysUser {
        id: user.id,
        account: user.account,
        password: None,
        name: user.name,
        email: user.email,
        avatar: user.avatar,
        login_check: user.login_check,
        state: user.state,
        balance: None,
        create_date: user.create_date,
        user_level: user.user_level,
    })))
}

pub async fn update_profile(
    State(context): State<Arc<ServiceContext>>,
    jwt_auth: JwtAuth,
    Json(req): Json<UpdateProfileRequest>,
) -> ApplicationResult<Json<ApiResponse<serde_json::Value>>> {
    let mut users =
        SysUser::select_by_map(context.rb.as_ref(), rbs::value! {"id": jwt_auth.id.clone()})
            .await?;
    let mut user = users
        .drain(..)
        .next()
        .ok_or_else(|| ApplicationError::NotFound {
            message: "用户不存在".to_string(),
            resource: Some("sys_user".to_string()),
            id: Some(jwt_auth.id.clone()),
        })?;

    if let Some(name) = req.name {
        user.name = Some(name.trim().to_string());
    }
    if let Some(email) = req.email {
        user.email = Some(email.trim().to_lowercase());
    }
    if let Some(avatar) = req.avatar {
        user.avatar = Some(avatar);
    }

    SysUser::update_by_map(
        context.rb.as_ref(),
        &user,
        rbs::value! {"id": jwt_auth.id.clone()},
    )
    .await?;
    Ok(Json(ApiResponse::success(
        serde_json::json!({"updated": true}),
    )))
}
