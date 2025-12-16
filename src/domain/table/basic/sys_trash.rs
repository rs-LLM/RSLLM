// 用途：导入执行器接口
// 说明：用于执行SQL查询，如删除过期的回收站记录
use rbatis::executor::Executor;
// 用途：导入日期时间类型
// 说明：用于记录回收站记录的创建时间
use rbatis::rbdc::DateTime;
// 用途：导入rbatis的CRUD宏和HTML SQL查询宏
// 说明：用于自动生成增删改查操作和HTML SQL查询方法
use rbatis::{crud, htmlsql};

// 用途：派生克隆、调试、序列化和反序列化特性
// 说明：支持克隆和调试，便于开发和测试；支持JSON序列化和反序列化，方便在网络中传输
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
// 用途：系统回收站表结构体
// 说明：用于存储被删除的数据，支持数据恢复功能
pub struct SysTrash {
    // 用途：回收站记录ID
    // 说明：唯一标识一条回收站记录
    pub id: Option<String>,
    // 用途：原表名
    // 说明：标识数据来自哪个表，用于恢复数据时确定目标表
    pub table_name: Option<String>,
    // 用途：被删除的数据
    // 说明：以JSON字符串形式存储被删除的数据，便于恢复
    pub data: Option<String>,
    // 用途：创建时间
    // 说明：记录数据被删除的时间，用于定期清理过期数据
    pub create_date: Option<DateTime>,
}

// 用途：生成CRUD操作宏
// 说明：自动实现增删改查等基本操作
crud!(SysTrash {});

// 用途：SysTrash实现
// 说明：提供自定义的删除方法
impl SysTrash {
    // 用途：根据时间删除过期的回收站记录
    // 说明：定期清理超过保留期限的回收站数据，释放存储空间
    htmlsql!(delete_by_day_before(rb:&dyn Executor, before:DateTime) -> rbatis::rbdc::db::ExecResult => "src/domain/table/basic/sys_trash.html");
}
