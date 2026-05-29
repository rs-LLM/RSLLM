## 2026-02-21 Task: bootstrap
- rsllm 统计服务已存在 admin/user 双轨路由与 service。
- 现有实现以 select_all/select_by_map + 内存聚合为主，精度与维度可增强。

## 2026-02-21 Task: aggregation-enhancement
- admin/user 统计 VO 扩展后，service 初始化必须同步填充所有字段，否则会触发 E0063（缺字段）编译错误。
- 维度聚合采用“主排序 request_count desc + 次排序 key asc”可保证前端展示稳定，不会因哈希遍历抖动。
- 前端 home 页在保持旧字段兼容前提下可渐进接入新字段；先扩展 API 类型再改 UI，可避免脚本类型漂移。

## 2026-02-21 Task: admin_stats_service 增强
- AdminOverviewStatsVO 新增字段：total_requests, successful_requests, failed_requests, success_rate, avg_response_time_ms, model_summary, api_key_summary, error_summary
- AdminTrendStatsVO 新增字段：success_trend, failure_trend, success_rate_trend, avg_response_time_trend
- AdminUserStatsVO 新增字段：total_requests, successful_requests, failed_requests, success_rate, avg_response_time_ms, total_consumption, top_consumers
- 聚合方法：aggregate_by_model, aggregate_by_api_key, aggregate_errors, aggregate_top_consumers
- 排序稳定：主指标降序 + key 升序（then_with）
- 除零安全：所有 success_rate 和 avg_response_time_ms 均检查分母 > 0
- API key 脱敏：空字符串返回 "(empty)"，短字符串返回 "*"，长字符串返回 "xxxx...xxxx"
- TopN 限制：model/api_key 摘要限制 20 条，error 摘要限制 10 条，top_consumers 限制 10 条

## 2026-02-21 Task: user_stats_service 增强
- UserStatsVO 新增字段：total_requests, successful_requests, failed_requests, success_rate, avg_response_time_ms, model_summary, provider_summary, request_type_summary
- UserTrendStatsVO 新增字段：request_trend, success_rate_trend, avg_response_time_trend
- UserTrendDataPointVO 已有字段：request_count, successful_count, failed_count, success_rate, avg_response_time_ms
- 聚合方法：aggregate_dimension（通用维度聚合，支持 key_extractor 闭包）
- provider 提取逻辑：优先从 extra.provider 获取，其次从 model_id 解析（按 '/' 或 ':' 分隔）
- TopN 限制：所有维度摘要限制 10 条
- 除零安全：calc_success_rate 和 calc_avg_response_time_ms 均检查分母 > 0
