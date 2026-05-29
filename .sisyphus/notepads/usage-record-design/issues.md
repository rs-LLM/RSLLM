## 2026-02-21 Task: bootstrap
- 前端监控已确定落在 rsllm-vue 的 home 页面。

## 2026-02-21 Task: verification-findings
- rsllm 全量 `cargo test` 仍有既有失败 2 项（plugin_router 404 预期 vs 500 实际），与本次 usage 聚合改造无直接代码耦合，但会影响“全绿”门禁。
- 本地前端手工 QA 时若未启动后端 8000 端口会出现 init/check 连接拒绝提示，属于环境依赖问题而非页面构建问题。

## 2026-02-21 Task: admin_stats_service 增强
- 潜在风险：select_all + 内存聚合在大数据量下可能有性能问题，建议后续考虑数据库聚合或分页加载
- 注意：model_name 和 username 字段当前为 None，需要前端自行关联或后续增强查询
- 注意：provider 维度聚合暂未实现，需通过 model_provider_mapping 关联，可作为后续增强点
- 注意：user_stats.rs 和 user_stats_service.rs 尚未增强，需单独任务处理
