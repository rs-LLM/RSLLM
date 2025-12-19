// 用途：导入全局上下文实例
// 说明：用于访问配置信息
use crate::context::CONTEXT;
// 用途：导入系统回收站表结构
// 说明：用于数据库操作和数据存储
use crate::domain::table::sys_trash::SysTrash;
// 用途：导入数据库连接池
// 说明：用于获取数据库连接
use crate::pool;
// 用途：导入互斥锁
// 说明：用于线程安全地访问和修改回收站清理日期
use parking_lot::Mutex;
// 用途：导入异步trait宏
// 说明：用于定义异步拦截器接口
use rbatis::async_trait;
// 用途：导入执行器接口
// 说明：用于执行SQL查询
use rbatis::executor::Executor;
// 用途：导入拦截器相关类型
// 说明：用于实现SQL拦截功能
use rbatis::intercept::{Intercept, ResultType};
// 用途：导入对象ID生成器
// 说明：用于生成回收站记录的唯一ID
use ulid::Ulid;
// 用途：导入日期时间类型
// 说明：用于记录回收站记录的创建时间
use rbatis::rbdc::DateTime;
// 用途：导入数据库错误类型
// 说明：用于错误处理
use rbatis::rbdc::Error;
// 用途：导入执行结果类型
// 说明：用于返回SQL执行结果
use rbatis::rbdc::db::ExecResult;
// 用途：导入rbs值类型
// 说明：用于处理数据库查询结果
use rbs::Value;
// 用途：导入序列化trait
// 说明：用于将数据序列化为JSON存储到回收站
use serde::Serialize;
// 用途：导入SQL解析相关类型
// 说明：用于解析和修改SQL语句
use sqlparser::ast::{FromTable, Statement};
// 用途：导入通用SQL方言
// 说明：用于SQL解析
use sqlparser::dialect::GenericDialect;
// 用途：导入SQL解析器
// 说明：用于解析SQL语句
use sqlparser::parser::Parser;
// 用途：导入Debug trait
// 说明：用于调试打印服务实例
use std::fmt::Debug;
// 用途：导入时间间隔类型
// 说明：用于计算时间差，判断是否需要清理回收站
use std::time::Duration;
// 用途：导入Arc类型
// 说明：用于包装Mutex以支持Clone
use std::sync::Arc;

// 用途：系统回收站服务结构体
// 说明：封装回收站的业务逻辑，包括数据回收、清理和SQL拦截
#[derive(Debug, Clone)]
pub struct SysTrashService {
    // 用途：上次清理时间
    // 说明：用于定期清理超过保留期限的回收站数据
    pub recycle_date: Arc<Mutex<DateTime>>,
}

// 用途：SysTrashService实现
// 说明：提供回收站服务的核心功能
impl SysTrashService {
    // 用途：创建回收站服务实例
    // 说明：初始化回收站服务，设置默认的清理日期
    pub fn new() -> Self {
        Self {
            recycle_date: Arc::new(Mutex::new(DateTime::now())),
        }
    }
    
    // 用途：添加数据到回收站
    // 说明：将被删除的数据保存到回收站，以便后续恢复或查看
    pub async fn add<T>(&self, table_name: &str, args: &[T]) -> Result<u64, Error>
    where
        T: Serialize,
    {
        // 用途：检查参数是否为空
        // 说明：避免空数据处理
        if args.is_empty() {
            return Ok(0);
        }
        // 用途：获取当前时间
        // 说明：用于记录回收站记录的创建时间
        let now = DateTime::now();
        // 用途：创建回收站记录向量
        // 说明：批量保存回收站记录，提高性能
        let mut trashes = Vec::with_capacity(args.len());
        // 用途：遍历数据，创建回收站记录
        // 说明：将每条数据转换为回收站记录
        for x in args {
            trashes.push(SysTrash {
                id: Some(Ulid::new().to_string().into()),
                table_name: Some(table_name.to_string()),
                data: Some(serde_json::to_string(x).unwrap_or_default()),
                create_date: Some(now.clone()),
            });
        }
        // 用途：批量插入回收站记录
        // 说明：提高插入性能
        let r = SysTrash::insert_batch(pool!(), &trashes, 20)
            .await?
            .rows_affected;
        // 用途：检查是否需要清理回收站
        // 说明：定期清理超过保留期限的数据，避免存储空间过大
        let diff = now.clone().0 - self.recycle_date.lock().0.clone();
        if diff > Duration::from_secs(24 * 3600) {
            *self.recycle_date.lock() = now.clone();
            let _ = self.recycle().await;
        }
        // 用途：返回插入的记录数
        // 说明：告知调用者插入结果
        Ok(r)
    }

    // 用途：清理超过保留期限的回收站数据
    // 说明：自动清理旧数据，释放存储空间
    pub async fn recycle(&self) -> Result<u64, Error> {
        // 用途：计算保留期限的截止时间
        // 说明：根据配置的保留天数计算需要清理的数据范围
        let before = DateTime::now().0.sub(Duration::from_secs(
            CONTEXT.config.trash_recycle_days * 24 * 3600,
        ));
        // 用途：删除超过保留期限的数据
        // 说明：清理旧数据，释放存储空间
        let r = SysTrash::delete_by_day_before(pool!(), DateTime(before)).await?;
        // 用途：返回删除的记录数
        // 说明：告知调用者清理结果
        Ok(r.rows_affected)
    }
}

// 用途：SQL拦截器实现
// 说明：拦截删除SQL，将数据保存到回收站后再执行删除操作
#[async_trait]
impl Intercept for SysTrashService {
    // 用途：SQL执行前的拦截处理
    // 说明：在删除数据前，将数据保存到回收站
    async fn before(
        &self,
        _task_id: i64,
        rb: &dyn Executor,
        sql: &mut String,
        args: &mut Vec<Value>,
        _result: ResultType<&mut Result<ExecResult, Error>, &mut Result<Vec<Value>, Error>>,
    ) -> Result<Option<bool>, Error> {
        // 用途：检查是否为删除语句
        // 说明：只处理删除操作
        if sql.starts_with("delete from ") {
            // 用途：创建通用SQL方言
            // 说明：用于解析SQL语句
            let dialect = GenericDialect {}; 
            // 用途：解析SQL语句
            // 说明：获取删除操作的表名
            let mut v: Vec<Statement> = Parser::parse_sql(&dialect, &sql.clone())
                .map_err(|e| Error::from(e.to_string()))?;
            // 用途：检查解析结果是否为空
            // 说明：确保SQL语句有效
            if v.len() <= 0 {
                return Err(Error::from("sql is empty"));
            }
            // 用途：提取表名
            // 说明：确定要删除数据的表
            let table = match v.remove(0) {
                Statement::Delete { from, .. } => {
                    let mut data = "".to_string();
                    match from {
                        FromTable::WithFromKeyword(v) => {
                            for x in v {
                                let x_str = &format!("{}", x);
                                data.push_str(x_str.as_str());
                            }
                        }
                        FromTable::WithoutKeyword(v) => {
                            for x in v {
                                let x_str = &format!("{}", x);
                                data.push_str(x_str.as_str());
                            }
                        }
                    }
                    data
                }
                _ => "".to_string(),
            };
            // 用途：检查表名是否为空
            // 说明：确保表名有效
            if table.is_empty() {
                return Err(Error::from(format!("sql={} table_name is empty", sql)));
            }
            // 用途：检查是否为回收站表
            // 说明：避免递归处理回收站表的删除操作
            if table.eq("sys_trash") {
                return Ok(Some(true));
            }
            // 用途：将删除语句转换为查询语句
            // 说明：在删除前先查询要删除的数据
            let new_sql = sql.clone().replace(
                &format!("delete from {}", table),
                &format!("select * from {}", table),
            );
            // 用途：执行查询，获取要删除的数据
            // 说明：将数据保存到回收站
            let data = rb.query(&new_sql, args.clone()).await?;
            // 用途：处理查询结果
            // 说明：将数据保存到回收站
            match data {
                Value::Array(arr) => {
                    self.add(&table, &arr).await?;
                }
                _ => {
                    return Err(Error::from(format!("data={} not array", data)));
                }
            }
        }
        // 用途：返回继续执行SQL的信号
        // 说明：允许SQL继续执行
        Ok(Some(true))
    }
}