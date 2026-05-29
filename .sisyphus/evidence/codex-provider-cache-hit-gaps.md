# Self Review - Gap Classification

## Critical

1. **User Scope 来源不稳定风险**
   - 若请求中无法稳定提取 user_scope（如 user_id 缺失），则 key 会抖动，命中率大幅下降。
   - 需要在策略层定义明确降级顺序（显式 user_id > auth identity > fallback anonymous scope）。

2. **流式/非流式注入不一致风险**
   - 若只在 non-stream 注入 `prompt_cache_key`/headers，会造成行为分叉。
   - 必须保证两条路径统一进入同一注入逻辑。

3. **并发下重复创建 cache id 风险**
   - 高并发同 key 时若仅读后写无二次检查，会产生多个 ID，破坏会话连续性。
   - 需要双检锁（read-check -> write-check -> create）。

## Minor

1. **TTL 参数默认值与运维感知**
   - TTL 过短影响命中，过长影响内存；需设置合理默认值并可配置。

2. **日志维度不足**
   - 仅有 hit/miss 不足以定位问题，建议增加 key 维度脱敏标签（provider/model/stream）。

3. **模块边界漂移**
   - 若将 provider 特定逻辑写入通用层，会降低后续多供应商可扩展性。

## Ambiguous

1. **Key 维度是否纳入更多请求参数**
   - 当前 MVP 为 provider+model+user_scope；是否扩展到 system/tools/temperature 尚未定案。

2. **是否在首版引入 singleflight**
   - 当前计划以读写锁双检满足首版；是否进一步做请求级合并待实测。

3. **header 字段规范性边界**
   - `Conversation_id`/`Session_id` 是否始终同值，是否存在供应商端差异仍需灰度验证。
