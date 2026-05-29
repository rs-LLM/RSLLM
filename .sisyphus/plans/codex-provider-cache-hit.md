# Plan: rsllm Codex 供应商缓存命中（Prompt/Session Cache）

## 目标与边界

- 目标：在 rsllm 中实现 **Codex 供应商缓存透传/复用**，提升供应商侧文本补全缓存命中。
- 首版范围：仅 Codex；注入并复用 `prompt_cache_key`、`Conversation_id`、`Session_id`。
- 非目标：
  - 不做本地完整响应结果缓存（不做“直接返回历史 completion 文本”）。
  - 不引入重型新依赖。

## 并发与一致性约束

- 同一 cache lookup key 在并发下应尽可能复用同一个 cache id。
- 使用并发安全内存存储（读写锁 + 双检创建）。
- 支持 TTL 过期与定时清理，避免内存无限增长。
- 流式与非流式请求的注入语义保持一致。

## 执行任务清单

### Wave 1 - 缓存抽象与数据结构（src/cache）
- [ ] 新增 `src/cache/mod.rs` 并导出 provider cache 子模块
- [ ] 新增 `src/cache/provider_cache/traits.rs`：定义策略接口（供应商可插拔）
- [ ] 新增 `src/cache/provider_cache/store.rs`：并发安全 TTL 存储（内存）
- [ ] 新增 `src/cache/provider_cache/types.rs`：cache entry、key 组成与上下文类型

### Wave 2 - Codex 策略实现
- [ ] 新增 `src/cache/provider_cache/codex.rs`：Codex key 规则、命中/创建逻辑
- [ ] 支持 key 至少包含：provider + model + user_scope（可扩展）
- [ ] 命中/未命中路径均输出统一结果（用于后续注入）

### Wave 3 - 请求注入集成（Provider 路径）
- [ ] 在 Codex 请求发送前集成策略调用（推荐接入 `src/providers/common/provider.rs` 的请求构建点）
- [ ] 注入请求体 `prompt_cache_key`
- [ ] 注入请求头 `Conversation_id` 与 `Session_id`
- [ ] 确保 stream / non-stream 两条路径行为一致

### Wave 4 - 配置与可观测性
- [ ] 新增最小配置项（如 `codex_prompt_cache_enabled`、`codex_prompt_cache_ttl_seconds`）
- [ ] 增加日志打点：`cache_hit` / `cache_miss` / `cache_set` / `cache_expired`
- [ ] 默认值与异常分支具备安全兜底（关闭缓存不影响现有调用）

### Wave 5 - 测试与回归
- [ ] 单测：TTL 过期、并发复用同 key、key 稳定性
- [ ] Provider 层测试：验证 body/header 注入字段正确
- [ ] 回归：非 Codex provider 不受影响
- [ ] 回归：stream/non-stream 不崩溃且注入一致

## 验证矩阵

- `cargo check`：编译通过
- `cargo test`：全量测试通过
- 目标测试（若已新增）：
  - `cargo test codex`
  - `cargo test cache`
- 并发验证：构造同 key 多并发请求，确认 cache id 复用率符合预期

## 风险清单（实现阶段重点关注）

- 用户标识缺失导致 key 抖动（需要 user_scope 兜底策略）
- 流式路径遗漏注入字段导致行为不一致
- TTL 清理与并发写入竞态（需双检/锁粒度设计）
- 仅做 Codex 时的分支隔离，避免影响其他 provider
