// 用途：导入全局上下文
// 说明：用于访问缓存服务
use crate::context::CONTEXT;

// 用途：导入字典相关的数据传输对象
// 说明：用于接收字典的编辑和分页查询请求参数
use crate::domain::dto::{DictEditDTO, DictPageDTO};

// 用途：导入字典表结构
// 说明：用于数据库操作
use crate::domain::table::sys_dict::SysDict;

// 用途：导入字典VO
// 说明：用于返回字典数据
use crate::domain::vo::SysDictVO;

// 用途：导入自定义错误类型
// 说明：用于处理错误情况
use crate::error::Error;

// 用途：导入自定义结果类型
// 说明：用于统一错误处理
use crate::error::Result;

// 用途：导入错误信息宏和数据库连接池宏
// 说明：用于生成错误信息和获取数据库连接
use crate::{error_info, pool};

// 用途：导入分页相关类型
// 说明：用于处理分页查询
use rbatis::{Page, PageRequest};

// 用途：导入rbs的value宏
// 说明：用于构建查询条件
use rbs::value;

/// 用途：字典缓存键
/// 说明：用于缓存所有字典数据
const DICT_KEY: &'static str = "sys_dict:all";

/// 用途：字典服务
/// 说明：处理字典相关业务逻辑
#[derive(Clone)]
pub struct SysDictService {}

impl SysDictService {
    /// 用途：分页查询字典
    /// 说明：从数据库中分页获取字典数据
    pub async fn page(&self, arg: &DictPageDTO) -> Result<Page<SysDictVO>> {
        // 用途：查询字典分页数据
        // 说明：根据查询条件从数据库中获取分页数据
        let page = SysDict::select_page(pool!(), &PageRequest::from(arg), arg).await?;
        // 用途：转换为VO分页
        // 说明：将数据库实体转换为前端需要的VO
        let page_vo = Page::<SysDictVO>::from(page);
        // 用途：返回分页结果
        // 说明：告知调用者查询成功并返回数据
        Ok(page_vo)
    }

    /// 用途：添加字典
    /// 说明：向数据库中添加新字典
    pub async fn add(&self, arg: &SysDict) -> Result<u64> {
        // 用途：检查字典是否已存在
        // 说明：避免重复添加字典
        let old = SysDict::select_by_map(pool!(), value! {"id":arg.id.as_deref().unwrap_or_default()}).await?;
        // 用途：如果字典已存在，返回错误
        // 说明：确保字典的唯一性
        if old.len() > 0 {
            return Err(Error::from(format!(
                "{},code={}",
                error_info!("dict_exists"),
                arg.code.as_deref().unwrap_or_default()
            )));
        }
        // 用途：插入字典数据
        // 说明：将新字典保存到数据库
        let result = Ok(SysDict::insert(pool!(), &arg).await?.rows_affected);
        // 用途：更新字典缓存
        // 说明：确保缓存中的字典数据与数据库一致
        self.update_cache().await?;
        // 用途：返回插入结果
        // 说明：告知调用者添加成功
        result
    }

    /// 用途：编辑字典
    /// 说明：更新数据库中的字典数据
    pub async fn edit(&self, arg: &DictEditDTO) -> Result<u64> {
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let data = SysDict::from(arg);
        // 用途：更新字典数据
        // 说明：根据ID更新字典信息
        let result = SysDict::update_by_map(pool!(), &data, value! {"id": &data.id }).await;
        // 用途：如果更新成功，更新缓存
        // 说明：确保缓存中的字典数据与数据库一致
        if result.is_ok() {
            self.update_cache().await?;
        }
        // 用途：返回更新结果
        // 说明：告知调用者更新成功
        Ok(result?.rows_affected)
    }

    /// 用途：删除字典
    /// 说明：从数据库中删除指定ID的字典
    pub async fn remove(&self, id: &str) -> Result<u64> {
        // 用途：删除字典数据
        // 说明：根据ID删除字典
        let r = SysDict::delete_by_map(pool!(), value! {"id": id }).await?;
        // 用途：如果删除成功，更新缓存
        // 说明：确保缓存中的字典数据与数据库一致
        if r.rows_affected > 0 {
            self.update_cache().await?;
        }
        // 用途：返回删除结果
        // 说明：告知调用者删除成功
        Ok(r.rows_affected)
    }

    /// 用途：更新所有字典缓存
    /// 说明：将数据库中的所有字典数据同步到缓存
    pub async fn update_cache(&self) -> Result<()> {
        // 用途：查询所有字典数据
        // 说明：获取最新的字典数据
        let all = SysDict::select_all(pool!()).await?;
        // 用途：更新缓存
        // 说明：将最新的字典数据存储到缓存
        CONTEXT.cache_service.set_json(DICT_KEY, &all).await?;
        // 用途：返回成功结果
        // 说明：告知调用者缓存更新成功
        Ok(())
    }
}
