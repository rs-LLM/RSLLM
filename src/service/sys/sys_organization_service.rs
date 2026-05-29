// 用途：导入全局上下文
// 说明：用于访问缓存服务
use crate::context::CONTEXT;

// 用途：导入组织相关的数据传输对象
// 说明：用于接收组织的编辑和分页查询请求参数
use crate::domain::dto::basic::sys_organization::{OrgAddDTO, OrgEditDTO, OrgPageDTO};

// 用途：导入组织表结构
// 说明：用于数据库操作
use crate::domain::table::basic::sys_organization::SysOrganization;

// 用途：导入组织视图对象
// 说明：用于返回组织数据
use crate::domain::vo::basic::sys_organization::{OrganizationTreeNodeVO, SysOrganizationVO};

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

/// 用途：组织缓存键
/// 说明：用于缓存所有组织数据
const ORG_KEY: &str = "sys_organization:all";

/// 用途：组织服务
/// 说明：处理组织相关业务逻辑
#[derive(Clone)]
pub struct SysOrganizationService {}

impl SysOrganizationService {
    /// 用途：分页查询组织
    /// 说明：从数据库中分页获取组织数据
    pub async fn page(&self, arg: &OrgPageDTO) -> Result<Page<SysOrganizationVO>> {
        // 用途：查询组织分页数据
        // 说明：根据查询条件从数据库中获取分页数据
        let page = SysOrganization::select_page(pool!(), &PageRequest::from(arg), arg).await?;
        // 用途：转换为视图对象分页
        // 说明：将数据库实体转换为前端需要的视图对象
        let page_vo = Page::<SysOrganizationVO>::from(page);
        // 用途：返回分页结果
        // 说明：告知调用者查询成功并返回数据
        Ok(page_vo)
    }

    /// 用途：添加组织
    /// 说明：向数据库中添加新组织
    pub async fn add(&self, arg: &OrgAddDTO) -> Result<u64> {
        // 用途：检查组织代码是否已存在
        // 说明：避免重复添加组织代码
        let old = SysOrganization::select_by_map(
            pool!(),
            value! {"code":arg.code.as_deref().unwrap_or_default()},
        )
        .await?;
        // 用途：如果组织代码已存在，返回错误
        // 说明：确保组织代码的唯一性
        if !old.is_empty() {
            return Err(Error::from(format!(
                "{},code={}",
                error_info!("org_exists"),
                arg.code.as_deref().unwrap_or_default()
            )));
        }
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let data = SysOrganization::from(arg.clone());
        // 用途：插入组织数据
        // 说明：将新组织保存到数据库
        let result = Ok(SysOrganization::insert(pool!(), &data).await?.rows_affected);
        // 用途：更新组织缓存
        // 说明：确保缓存中的组织数据与数据库一致
        self.update_cache().await?;
        // 用途：返回插入结果
        // 说明：告知调用者添加成功
        result
    }

    /// 用途：编辑组织
    /// 说明：更新数据库中的组织数据
    pub async fn edit(&self, arg: &OrgEditDTO) -> Result<u64> {
        // 用途：转换为数据库实体
        // 说明：数据库操作需要使用实体对象
        let data = SysOrganization::from(arg);
        // 用途：更新组织数据
        // 说明：根据ID更新组织信息
        let result = SysOrganization::update_by_map(pool!(), &data, value! {"id": &data.id }).await;
        // 用途：如果更新成功，更新缓存
        // 说明：确保缓存中的组织数据与数据库一致
        if result.is_ok() {
            self.update_cache().await?;
        }
        // 用途：返回更新结果
        // 说明：告知调用者更新成功
        Ok(result?.rows_affected)
    }

    /// 用途：删除组织
    /// 说明：从数据库中删除指定ID的组织
    pub async fn remove(&self, id: &str) -> Result<u64> {
        // 用途：删除组织数据
        // 说明：根据ID删除组织
        let r = SysOrganization::delete_by_map(pool!(), value! {"id": id }).await?;
        // 用途：如果删除成功，更新缓存
        // 说明：确保缓存中的组织数据与数据库一致
        if r.rows_affected > 0 {
            self.update_cache().await?;
        }
        // 用途：返回删除结果
        // 说明：告知调用者删除成功
        Ok(r.rows_affected)
    }

    /// 用途：查询所有组织
    /// 说明：获取所有组织数据
    pub async fn finds_all(&self) -> Result<Vec<SysOrganizationVO>> {
        // 用途：查询所有组织数据
        // 说明：从数据库中获取所有组织
        let all = SysOrganization::select_all_custom(pool!()).await?;
        // 用途：转换为视图对象列表
        // 说明：将数据库实体转换为视图对象
        let vo_list: Vec<SysOrganizationVO> =
            all.into_iter().map(SysOrganizationVO::from).collect();
        // 用途：返回视图对象列表
        // 说明：告知调用者查询成功并返回数据
        Ok(vo_list)
    }

    /// 用途：查询组织详情
    /// 说明：根据ID获取组织详细信息
    pub async fn detail(&self, id: &str) -> Result<SysOrganizationVO> {
        // 用途：查询组织详情
        // 说明：根据ID从数据库中获取组织数据
        let org = SysOrganization::select_by_map(pool!(), value! {"id": id})
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| Error::from("组织不存在"))?;
        // 用途：转换为视图对象
        // 说明：将数据库实体转换为视图对象
        let vo = SysOrganizationVO::from(org);
        // 用途：返回视图对象
        // 说明：告知调用者查询成功并返回数据
        Ok(vo)
    }

    /// 用途：查询组织树
    /// 说明：获取组织的树形结构
    pub async fn find_tree(&self) -> Result<Vec<OrganizationTreeNodeVO>> {
        // 用途：查询所有组织数据
        // 说明：从数据库中获取所有组织
        let all = SysOrganization::select_all_custom(pool!()).await?;
        // 用途：构建组织树
        // 说明：将扁平的组织列表转换为树形结构
        let tree = self.build_org_tree(all);
        // 用途：返回组织树
        // 说明：告知调用者查询成功并返回数据
        Ok(tree)
    }

    /// 用途：构建组织树
    /// 说明：将扁平的组织列表转换为树形结构
    fn build_org_tree(&self, orgs: Vec<SysOrganization>) -> Vec<OrganizationTreeNodeVO> {
        // 用途：创建组织节点映射
        // 说明：将所有组织转换为节点，并按ID建立索引
        let mut org_map: std::collections::HashMap<String, OrganizationTreeNodeVO> =
            std::collections::HashMap::new();
        // 用途：记录所有组织的parent_id
        // 说明：用于判断是否为顶级组织
        let mut parent_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        // 用途：记录组织的父子关系
        // 说明：用于构建树形结构
        let mut org_relations: Vec<(String, Option<String>)> = Vec::new();

        // 用途：遍历所有组织，构建组织节点
        // 说明：将每个组织转换为树节点
        for org in &orgs {
            let org_id = org.id.clone().unwrap_or_default();
            let node = OrganizationTreeNodeVO {
                id: org.id.clone(),
                name: org.name.clone(),
                code: org.code.clone(),
                org_type: org.org_type.clone(),
                sort_order: org.sort_order,
                children: vec![],
            };

            // 用途：将组织节点添加到映射中
            // 说明：使用ID作为键
            org_map.insert(org_id.clone(), node);

            // 用途：记录parent_id
            // 说明：用于判断是否为顶级组织
            if let Some(parent_id) = org.parent_id.clone() {
                parent_ids.insert(parent_id);
            }

            // 用途：记录父子关系
            // 说明：保存组织ID和父组织ID
            org_relations.push((org_id, org.parent_id.clone()));
        }

        // 用途：构建父子关系
        // 说明：根据记录的父子关系建立组织的层级关系
        for (org_id, parent_id) in org_relations {
            // 用途：检查是否有父组织
            // 说明：如果parent_id不为空，则建立父子关系
            if let Some(parent_id) = parent_id {
                // 用途：先从映射中移除子节点
                // 说明：获取子节点的所有权
                let child_node = org_map.remove(&org_id);
                // 用途：获取父节点
                // 说明：从映射中获取父节点的可变引用
                if let Some(parent_node) = org_map.get_mut(&parent_id) {
                    // 用途：将当前组织添加到父组织的子列表中
                    // 说明：建立父子关系
                    if let Some(child) = child_node {
                        parent_node.children.push(child);
                    }
                }
            }
        }

        // 用途：提取顶级组织
        // 说明：返回没有父组织的顶级组织（ID不在parent_ids集合中的组织）
        org_map
            .into_values()
            .filter(|org| {
                if let Some(id) = org.id.as_ref() {
                    !parent_ids.contains(id)
                } else {
                    true
                }
            })
            .collect()
    }

    /// 用途：更新所有组织缓存
    /// 说明：将数据库中的所有组织数据同步到缓存
    pub async fn update_cache(&self) -> Result<()> {
        // 用途：查询所有组织数据
        // 说明：获取最新的组织数据
        let all = SysOrganization::select_all_custom(pool!()).await?;
        // 用途：更新缓存
        // 说明：将最新的组织数据存储到缓存
        CONTEXT.cache_service.set_json(ORG_KEY, &all).await?;
        // 用途：返回成功结果
        // 说明：告知调用者缓存更新成功
        Ok(())
    }
}
