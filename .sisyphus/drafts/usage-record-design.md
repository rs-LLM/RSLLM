# Draft: rsllm 用量记录设计（参考 CLIProxyAPI）

## Requirements (confirmed)
- 用户目标：在 `rsllm` 项目中，参考 `CLIProxyAPI` 的方案来设计“用量记录”能力。
- 用户动作：已明确要求“请开始”，表示进入需求澄清与规划阶段。
- 用户确认：不需要新增写入链路，重点是“聚合统计能力升级”。
- 用户确认：沿用现有 usage 表并“加字段”，实现更精细维度拆分。
- 用户确认：测试策略为“实现后补充测试”。
- 用户偏好：尽可能完整覆盖能力，尤其要有美观的监控统计前端，并区分管理员与用户视角。

## Technical Decisions
- 当前阶段：先做方案访谈与边界确认，不直接实现代码。
- 参考策略：以 CLIProxyAPI 的“采集插件 + 聚合统计 + 管理端消费”思路作为设计参考，而非盲目照搬。
- 写入策略：默认不新增写入入口，优先复用现有用量写入来源（如 billing/usage 现有流程），聚焦统计层与展示层。
- 统计范围：按用户、管理员全局、API Key、Provider、Model、时间粒度（小时/天）进行精细拆分。

## Research Findings
- CLIProxyAPI 已有成熟 usage 模块：`internal/usage/logger_plugin.go`，具备开关、聚合、快照、去重合并等能力。
- CLIProxyAPI 管理端通过 `/v0/management/usage` 输出统计数据，并被 TUI 客户端消费。
- rsllm 已有 usage 的 DTO/VO/Table/Service/Controller/Router 骨架，且已有查询类接口（`/usage-logs` 等）。
- rsllm 当前主要是读与聚合链路；写入采集链路（统一入口）需进一步确认。
- rsllm 已有管理员统计路由与服务（`/admin/stats/*` + `admin_stats_service`）以及用户统计路由（`/usage-logs/*`、`/user/stats/*`），具备“管理员/用户维度分离”的技术基础。
- 仓库内未发现成熟前端工程（主要是 Rust 后端与少量 html/sql 模板），前端监控页面落地位置尚需明确。

## Open Questions
- 前端落地位置：
  - 方案A：在 `rsllm` 仓库内新增/扩展前端模块来做监控统计页面；
  - 方案B：在独立前端仓库实现，仅由 rsllm 提供统计 API；
  - 方案C：先只做后端 API 与契约，前端后续单独接入。
- “加字段”精确范围：是否包含请求头/路径、状态码分桶、错误分类、延迟分位数（P50/P95/P99）、缓存命中、模型版本等高维字段。

## Scope Boundaries
- INCLUDE: 用量聚合统计增强、统计维度扩展、管理员/用户视图分离、监控统计展示方案。
- EXCLUDE: 不新增独立写入链路（默认），除非后续确认现有链路无法满足新增字段。

## Test Strategy Decision
- **Infrastructure exists**: YES（项目内已有较多测试与路由/服务测试痕迹）
- **Automated tests**: YES（实现后补充）
- **If setting up**: N/A
- **Agent-Executed QA**: ALWAYS（前端用 Playwright，API 用 curl，CLI/TUI 用终端验证）
