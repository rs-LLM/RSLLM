// 用途：导入全局上下文
// 说明：用于访问配置和其他服务
use crate::context::CONTEXT;

// 用途：导入自定义错误类型
// 说明：用于处理错误情况
use crate::error::Error;

// 用途：导入自定义结果类型
// 说明：用于统一错误处理
use crate::error::Result;

// 用途：导入分页相关类型
// 说明：用于处理分页查询
use rbatis::page::{Page, PageRequest};

// 用途：导入日期时间类型
// 说明：用于处理时间相关操作
use rbatis::rbdc::DateTime;

// 用途：导入用户角色添加DTO
// 说明：用于添加用户角色关联
use crate::domain::dto::rbac::UserRoleAddDTO;

// 用途：导入用户相关的数据传输对象
// 说明：用于接收用户的各种请求参数
use crate::domain::dto::{IdDTO, SignInDTO, UserAddDTO, UserEditDTO, UserPageDTO, UserRolePageDTO};

// 用途：导入登录检查枚举
// 说明：用于处理不同的登录验证方式
use crate::domain::table::LoginCheck;

// 用途：导入用户表结构
// 说明：用于数据库操作
use crate::domain::table::sys_user::SysUser;

// 用途：导入用户VO
// 说明：用于返回用户数据
use crate::domain::vo::sys_user::SysUserVO;

// 用途：导入JWT令牌和登录VO
// 说明：用于处理登录和令牌相关操作
use crate::domain::vo::{JWTToken, SignInVO};

// 用途：导入设置用户VO
// 说明：用于设置用户角色
use crate::service::SetUserVO;

// 用途：导入密码加密工具
// 说明：用于处理密码的加密和验证
use crate::util::password_encoder::PasswordEncoder;

// 用途：导入错误信息宏和数据库连接池宏
// 说明：用于生成错误信息和获取数据库连接
use crate::{error_info, pool};

// 用途：导入rbs的value宏
// 说明：用于构建查询条件
use rbs::value;

// 用途：导入Duration类型
// 说明：用于设置缓存过期时间
use std::time::Duration;

/// 用途：登录重试缓存键
/// 说明：用于缓存登录失败重试次数
const CACHE_KEY_RETRY: &'static str = "login:login_retry";

/// 用途：后台用户服务
/// 说明：处理用户相关业务逻辑，如登录、注册、权限管理等
pub struct SysUserService {}

impl SysUserService {
    /// 用途：获取用户角色分页列表
    /// 说明：返回带有角色信息的用户分页数据
    pub async fn role_page(&self, dto: &UserRolePageDTO) -> Result<Page<SysUserVO>> {
        // 用途：调用page方法获取用户分页数据
        // 说明：先获取基础的用户分页数据
        let mut vo = CONTEXT
            .sys_user_service
            .page(UserPageDTO::from(dto))
            .await?;
        // 用途：创建角色列表
        // 说明：用于存储用户和角色的对应关系
        let mut roles = Vec::with_capacity(vo.records.len());
        // 用途：遍历用户记录
        // 说明：为每个用户创建一个SetUserVO对象
        for x in &vo.records {
            roles.push(SetUserVO {
                id: x.id.clone(),
                roles: vec![],
            });
        }
        // 用途：设置用户角色
        // 说明：为每个用户填充角色信息
        CONTEXT.rbac_user_role_service.set_roles(&mut roles).await?;
        // 用途：遍历角色列表
        // 说明：将角色信息赋值给对应的用户VO
        let mut idx = 0;
        for x in roles {
            vo.records[idx].roles = x.roles;
            idx += 1;
        }
        // 用途：返回带角色信息的用户分页数据
        // 说明：告知调用者查询成功并返回数据
        Ok(vo)
    }
}

impl SysUserService {
    /// 用途：获取用户分页列表
    /// 说明：从数据库中分页获取用户数据
    pub async fn page(&self, arg: UserPageDTO) -> Result<Page<SysUserVO>> {
        // 用途：查询用户分页数据
        // 说明：根据查询条件从数据库中获取分页数据
        let sys_user_page = SysUser::select_page(pool!(),&PageRequest::from(arg.clone()),&arg).await?;
        // 用途：转换为VO分页
        // 说明：将数据库实体转换为前端需要的VO
        let page = Page::<SysUserVO>::from(sys_user_page);
        // 用途：返回分页结果
        // 说明：告知调用者查询成功并返回数据
        Ok(page)
    }

    /// 用途：获取用户详情
    /// 说明：根据用户ID获取用户的详细信息
    pub async fn detail(&self, arg: &IdDTO) -> Result<SysUserVO> {
        // 用途：获取用户ID
        // 说明：从请求参数中提取用户ID
        let user_id = arg.id.as_deref().unwrap_or_default();
        // 用途：查找用户
        // 说明：根据用户ID从数据库中查询用户数据
        let user = self.find(&user_id).await?.ok_or_else(|| {
            Error::from(format!("{}={}", error_info!("user_not_exists"), user_id))
        })?;
        // 用途：转换为用户VO
        // 说明：将数据库实体转换为前端需要的VO
        let mut user_vo = SysUserVO::from(user);
        // 用途：获取用户角色
        // 说明：查询用户关联的角色信息
        let roles = CONTEXT
            .rbac_user_role_service
            .find_user_role(&user_id)
            .await?;
        // 用途：设置用户角色
        // 说明：将角色信息赋值给用户VO
        user_vo.roles = roles;
        // 用途：返回用户详情
        // 说明：告知调用者查询成功并返回数据
        Ok(user_vo)
    }

    /// 用途：根据ID查找用户
    /// 说明：根据用户ID从数据库中查询用户
    pub async fn find(&self, id: &str) -> Result<Option<SysUser>> {
        // 用途：根据ID查询用户
        // 说明：从数据库中查询指定ID的用户
        Ok(SysUser::select_by_map(pool!(), value! {"id":id})
            .await?
            .into_iter()
            .next())
    }

    /// 用途：根据账号查找用户
    /// 说明：根据用户账号从数据库中查询用户
    pub async fn find_by_account(&self, account: &str) -> Result<Option<SysUser>> {
        // 用途：根据账号查询用户
        // 说明：从数据库中查询指定账号的用户
        Ok(SysUser::select_by_map(pool!(), value! {"account": account})
            .await?
            .into_iter()
            .next())
    }

    /// 用途：添加用户
    /// 说明：向数据库中添加新用户
    pub async fn add(&self, mut arg: UserAddDTO) -> Result<u64> {
        // 用途：检查账号和名称是否为空
        // 说明：账号和名称是用户的必填项
        if arg.account.is_none()
            || arg.account.as_deref().unwrap_or_default().is_empty()
            || arg.name.is_none()
            || arg.name.as_deref().unwrap_or_default().is_empty()
        {
            return Err(Error::from(error_info!("user_and_name_cannot_empty")));
        }
        // 用途：检查账号是否已存在
        // 说明：避免重复添加相同账号的用户
        let old_user = self
            .find_by_account(arg.account.as_deref().unwrap_or_default())
            .await?;
        if old_user.is_some() {
            return Err(Error::from(format!(
                "用户账户:{}已存在!",
                arg.account.as_deref().unwrap_or_default()
            )));
        }
        // 用途：获取密码
        // 说明：处理用户密码
        let mut password = arg.password.as_deref().unwrap_or_default().to_string();
        // 用途：检查密码是否为空
        // 说明：密码为空时使用默认密码
        if password.is_empty() {
            // 用途：设置默认密码
            // 说明：确保新用户有一个初始密码
            password = "123456".to_string();
        }
        // 用途：设置密码
        // 说明：将密码赋值给DTO
        arg.password = Some(password);
        // 用途：获取角色ID
        // 说明：处理用户角色关联
        let role_id = arg.role_id.clone();
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let user = SysUser::from(arg);
        // 用途：添加用户角色关联
        // 说明：如果角色ID存在，为用户分配角色
        if role_id.is_some() {
            CONTEXT
                .rbac_user_role_service
                .add(UserRoleAddDTO {
                    id: None,
                    user_id: user.id.clone(),
                    role_id: role_id,
                })
                .await?;
        }
        // 用途：插入用户数据
        // 说明：将新用户数据保存到数据库
        Ok(SysUser::insert(pool!(), &user).await?.rows_affected)
    }

    /// 用途：用户登录
    /// 说明：处理用户登录请求，验证用户身份
    pub async fn sign_in(&self, arg: &SignInDTO) -> Result<SignInVO> {
        // 用途：检查是否需要等待登录
        // 说明：防止登录重试频率过高
        self.is_need_wait_login_ex(&arg.account).await?;
        // 用途：根据账号查询用户
        // 说明：获取用户信息用于验证
        let user: Option<SysUser> = SysUser::select_by_map(pool!(), value! {"account": &arg.account})
            .await?
            .into_iter()
            .next();
        // 用途：检查用户是否存在
        // 说明：用户不存在则登录失败
        let user = user.ok_or_else(|| {
            Error::from(format!(
                "{}={}",
                error_info!("account_not_exists"),
                arg.account
            ))
        })?;
        // 用途：检查用户是否被禁用
        // 说明：禁用用户无法登录
        if user.state.eq(&Some(0)) {
            return Err(Error::from(error_info!("account_disabled")));
        }
        // 用途：初始化错误变量
        // 说明：用于存储登录验证过程中的错误
        let mut error = None;
        // 用途：匹配登录检查类型
        // 说明：根据不同的登录检查类型执行不同的验证逻辑
        match user
            .login_check
            .as_ref()
            .unwrap_or(&LoginCheck::PasswordCheck)
        {
            // 用途：无检查
            // 说明：不需要任何验证
            LoginCheck::NoCheck => {
                // no check
            }
            // 用途：密码检查
            // 说明：只需要验证密码
            LoginCheck::PasswordCheck => {
                // 用途：检查密码
                // 说明：验证用户输入的密码是否正确
                if !PasswordEncoder::verify(
                    user.password
                        .as_ref()
                        .ok_or_else(|| Error::from(error_info!("password_empty")))?,
                    &arg.password,
                ) {
                    error = Some(Error::from(error_info!("password_error")));
                }
            }
            // 用途：密码+图片验证码检查
            // 说明：需要验证密码和图片验证码
            LoginCheck::PasswordImgCodeCheck => {
                // 用途：检查图片验证码
                // 说明：验证用户输入的验证码是否正确
                let cache_code = CONTEXT
                    .cache_service
                    .get_string(&format!("captch:account_{}", &arg.account))
                    .await?;
                if arg.vcode == ""
                    || cache_code
                        .to_lowercase()
                        .as_str()
                        .ne(arg.vcode.to_lowercase().as_str())
                {
                    error = Some(Error::from(error_info!("vcode_error")))
                }
                // 用途：检查密码
                // 说明：验证码正确后验证密码
                if error.is_none()
                    && !PasswordEncoder::verify(
                        user.password
                            .as_ref()
                            .ok_or_else(|| Error::from(error_info!("password_empty")))?,
                        &arg.password,
                    )
                {
                    error = Some(Error::from(error_info!("password_error")));
                }
            }
            // 用途：手机验证码检查
            // 说明：需要验证手机验证码
            LoginCheck::PhoneCodeCheck => {
                // 用途：获取短信验证码
                // 说明：从缓存中获取之前发送的验证码
                let sms_code = CONTEXT
                    .cache_service
                    .get_string(&format!(
                        "{}{}",
                        CONTEXT.config.sms_cache_send_key_prefix, &arg.account
                    ))
                    .await?;
                // 用途：检查短信验证码
                // 说明：验证用户输入的短信验证码是否正确
                if !sms_code.eq(&arg.vcode) {
                    error = Some(Error::from(error_info!("vcode_error")));
                }
            }
        }
        // 用途：检查是否有错误
        // 说明：有错误则记录重试次数并返回错误
        if let Some(e) = &error {
            self.add_retry_login_limit_num(&arg.account).await?;
            return Err(e.clone());
        }
        // 用途：获取用户信息
        // 说明：生成登录响应数据
        let sign_in_vo = self.get_user_info(&user).await?;
        // 用途：返回登录结果
        // 说明：告知调用者登录成功并返回用户信息和令牌
        Ok(sign_in_vo)
    }

    /// 用途：检查是否需要等待登录
    /// 说明：防止登录重试频率过高
    pub async fn is_need_wait_login_ex(&self, account: &str) -> Result<()> {
        // 用途：检查是否配置了登录重试限制
        // 说明：只有配置了重试限制才需要检查
        if CONTEXT.config.login_fail_retry > 0 {
            // 用途：获取重试次数
            // 说明：从缓存中获取登录失败重试次数
            let num: Option<u64> = CONTEXT
                .cache_service
                .get_json(&format!("{}{}", CACHE_KEY_RETRY, account))
                .await?;
            // 用途：检查重试次数是否超过限制
            // 说明：超过限制则需要等待
            if num.unwrap_or(0) >= CONTEXT.config.login_fail_retry {
                // 用途：获取缓存剩余过期时间
                // 说明：计算需要等待的时间
                let wait_sec: i64 = CONTEXT
                    .cache_service
                    .ttl(&format!("{}{}", CACHE_KEY_RETRY, account))
                    .await?;
                // 用途：检查剩余时间是否大于0
                // 说明：剩余时间大于0则需要等待
                if wait_sec > 0 {
                    // 用途：生成错误信息
                    // 说明：告知用户需要等待的时间
                    let mut e = error_info!("req_frequently");
                    e = e.replace("{}", &format!("{}", wait_sec));
                    return Err(Error::from(e));
                }
            }
        }
        // 用途：返回成功结果
        // 说明：告知调用者可以继续登录
        Ok(())
    }

    /// 用途：添加登录重试记录
    /// 说明：记录登录失败重试次数
    pub async fn add_retry_login_limit_num(&self, account: &str) -> Result<()> {
        // 用途：检查是否配置了登录重试限制
        // 说明：只有配置了重试限制才需要记录
        if CONTEXT.config.login_fail_retry > 0 {
            // 用途：获取当前重试次数
            // 说明：从缓存中获取当前重试次数
            let num: Option<u64> = CONTEXT
                .cache_service
                .get_json(&format!("{}{}", CACHE_KEY_RETRY, account))
                .await?;
            // 用途：计算新的重试次数
            // 说明：增加重试次数，不超过配置的最大值
            let mut num = num.unwrap_or(0);
            if num > CONTEXT.config.login_fail_retry {
                num = CONTEXT.config.login_fail_retry;
            }
            num += 1;
            // 用途：更新缓存
            // 说明：将新的重试次数存储到缓存
            CONTEXT
                .cache_service
                .set_string_ex(
                    &format!("{}{}", CACHE_KEY_RETRY, account),
                    &num.to_string(),
                    Some(Duration::from_secs(
                        CONTEXT.config.login_fail_retry_wait_sec,
                    )),
                )
                .await?;
        }
        // 用途：返回成功结果
        // 说明：告知调用者重试次数记录成功
        Ok(())
    }

    /// 用途：根据令牌获取用户信息
    /// 说明：验证令牌并返回用户信息
    pub async fn get_user_info_by_token(&self, token: &JWTToken) -> Result<SignInVO> {
        // 用途：根据ID查询用户
        // 说明：从数据库中获取用户信息
        let user = SysUser::select_by_map(pool!(), value! {"id": &token.id})
            .await?
            .into_iter()
            .next();
        // 用途：检查用户是否存在
        // 说明：用户不存在则令牌无效
        let user = user.ok_or_else(|| {
            Error::from(format!(
                "{}:{}",
                error_info!("account_not_exists"),
                token.account
            ))
        })?;
        // 用途：获取用户信息
        // 说明：生成登录响应数据
        self.get_user_info(&user).await
    }

    /// 用途：获取用户信息
    /// 说明：生成登录响应数据
    pub async fn get_user_info(&self, user: &SysUser) -> Result<SignInVO> {
        // 用途：复制用户对象
        // 说明：避免修改原对象
        let mut user = user.clone();
        // 用途：清除密码
        // 说明：返回给前端的数据中不应包含密码
        user.password = None;
        // 用途：获取用户ID
        // 说明：用于后续操作
        let user_id = user
            .id
            .clone()
            .ok_or_else(|| Error::from(error_info!("id_empty")))?;
        // 用途：转换为登录VO
        // 说明：生成登录响应数据
        let mut sign_vo = SignInVO::from(user);
        // 用途：加载用户权限
        // 说明：获取用户的所有权限
        sign_vo.permissions = self.load_level_permission(&user_id).await?;
        // 用途：创建JWT令牌
        // 说明：生成访问令牌
        let jwt_token = JWTToken {
            id: sign_vo.id.clone().unwrap_or_default(),
            account: sign_vo.account.clone().unwrap_or_default(),
            permissions: sign_vo.permissions.clone(),
            role_ids: vec![],
            exp: DateTime::now().unix_timestamp() as usize + CONTEXT.config.jwt_exp,
        };
        // 用途：生成访问令牌
        // 说明：使用JWT算法生成令牌
        sign_vo.access_token = jwt_token.create_token(&CONTEXT.config.jwt_secret)?;
        // 用途：获取用户角色
        // 说明：为登录响应添加角色信息
        sign_vo.roles = CONTEXT
            .rbac_user_role_service
            .find_user_role(&sign_vo.id.clone().unwrap_or_default())
            .await?;
        // 用途：返回登录VO
        // 说明：告知调用者获取成功并返回数据
        Ok(sign_vo)
    }

    /// 用途：用户登出
    /// 说明：处理用户登出请求
    pub async fn sign_out(&self) {}

    /// 用途：编辑用户
    /// 说明：更新用户信息
    pub async fn edit(&self, arg: UserEditDTO) -> Result<u64> {
        // 用途：获取角色ID
        // 说明：处理用户角色关联
        let role_id = arg.role_id.clone();
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let mut arg = SysUser::from(arg);
        // 用途：获取旧用户信息
        // 说明：用于比较密码是否需要更新
        let user = SysUser::select_by_map(pool!(), value! {"id": arg.id.as_ref()})
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from(error_info!("user_cannot_find")))?;
        // 用途：不更新账号
        // 说明：账号是用户的唯一标识，不应被修改
        arg.account = None;
        // 用途：初始化密码变量
        // 说明：用于存储加密后的密码
        let mut password = None;
        // 用途：检查密码是否被修改
        // 说明：密码被修改则需要重新加密
        if arg.password != user.password {
            if let Some(pass) = arg.password.as_ref() {
                // 用途：加密密码
                // 说明：将用户输入的密码加密后存储
                password = Some(PasswordEncoder::encode(pass));
            }
        }
        // 用途：设置密码
        // 说明：将加密后的密码赋值给实体
        arg.password = password;
        // 用途：添加用户角色关联
        // 说明：如果角色ID存在，为用户分配角色
        if role_id.is_some() {
            CONTEXT
                .rbac_user_role_service
                .add(UserRoleAddDTO {
                    id: None,
                    user_id: arg.id.clone(),
                    role_id: role_id,
                })
                .await?;
        }
        // 用途：更新用户数据
        // 说明：将更新后的用户数据保存到数据库
        Ok(SysUser::update_by_map(pool!(), &arg, value! {"id":&arg.id})
            .await?
            .rows_affected)
    }

    /// 用途：删除用户
    /// 说明：从数据库中删除指定ID的用户
    pub async fn remove(&self, id: &str) -> Result<u64> {
        // 用途：检查ID是否为空
        // 说明：ID为空则无法删除
        if id.is_empty() {
            return Err(Error::from(error_info!("id_empty")));
        }
        // 用途：删除用户数据
        // 说明：从数据库中删除用户
        let r = SysUser::delete_by_map(pool!(), value! {"id": id}).await?;
        // 用途：删除用户角色关联
        // 说明：级联删除用户的角色关联
        CONTEXT.rbac_user_role_service.remove_by_user_id(id).await?;
        // 用途：返回删除结果
        // 说明：告知调用者删除成功
        Ok(r.rows_affected)
    }

    /// 用途：查找用户权限层次结构
    /// 说明：获取用户的所有权限
    pub async fn load_level_permission(&self, user_id: &str) -> Result<Vec<String>> {
        // 用途：获取用户角色
        // 说明：通过角色获取权限
        let data = CONTEXT
            .rbac_user_role_service
            .find_user_role(user_id)
            .await?;
        // 用途：创建权限列表
        // 说明：用于存储用户的所有权限
        let mut perms = Vec::with_capacity(data.len());
        // 用途：遍历角色
        // 说明：获取每个角色的权限
        for x in data {
            // 用途：遍历权限
            // 说明：将所有权限添加到列表中
            for x in x.permissions {
                perms.push(x.permission.clone().unwrap_or_default());
            }
        }
        // 用途：返回权限列表
        // 说明：告知调用者查询成功并返回数据
        Ok(perms)
    }
}
