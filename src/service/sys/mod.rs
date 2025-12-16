
// 用途：声明系统认证服务模块
// 原因：处理认证相关业务逻辑，如权限验证
mod sys_auth_service;

// 用途：声明系统字典服务模块
// 原因：处理字典相关业务逻辑，如字典的增删改查
mod sys_dict_service;

// 用途：声明系统短信服务模块
// 原因：处理短信发送相关业务逻辑
mod sys_sms_service;

// 用途：声明系统回收站服务模块
// 原因：处理回收站相关业务逻辑，如数据的软删除和恢复
mod sys_trash_service;

// 用途：声明系统用户服务模块
// 原因：处理用户相关业务逻辑，如用户的登录、注册、信息管理
mod sys_user_service;

// 用途：导出系统认证服务
// 原因：允许其他模块访问认证功能
pub use sys_auth_service::*;

// 用途：导出系统字典服务
// 原因：允许其他模块访问字典功能
pub use sys_dict_service::*;

// 用途：导出系统短信服务
// 原因：允许其他模块访问短信发送功能
pub use sys_sms_service::*;

// 用途：导出系统回收站服务
// 原因：允许其他模块访问回收站功能
pub use sys_trash_service::*;

// 用途：导出系统用户服务
// 原因：允许其他模块访问用户相关功能
pub use sys_user_service::*;
