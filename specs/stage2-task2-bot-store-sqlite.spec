spec: task
name: "AITK Bot Store (SQLite)"
tags: [stage2, aitk, sqlite, storage]
---

## 意图

在 AITK 中实现基于 SQLite 的 Bot 数据持久化层，管理 Bot 元数据、消息历史
和 update 跟踪。该模块为 Telegram Bot API Server 提供存储后端，支持应用
重启后恢复 Bot 列表和消息历史。

## 约束

- 模块 gate 在 `cfg(not(target_arch = "wasm32"))` 下，feature flag
  `telegram-server`
- 使用 `rusqlite` crate（与 AITK 轻量级定位一致）
- 启动时执行 schema version 检查，支持自动迁移
- 库代码禁止 `.unwrap()`
- 所有公开类型必须有 doc comment 和 `Debug, Clone` derive
- SQLite 文件存储路径由调用方通过 `BotStoreConfig` 配置

## 已定决策

- ORM: 不用 ORM，直接用 rusqlite prepared statements
- schema_version 表记录当前版本号，启动时检查并迁移
- updates 表与 messages 表分离：updates 跟踪长轮询 offset，messages
  存储完整消息历史
- 媒体文件存储在 `{data_dir}/bot_media/{file_id}` 目录，media 表记录元信息
- update_id 为全局递增（跨 Bot），简化 offset 逻辑

## 边界

### 允许修改
- moly-aitk/src/telegram_server/store.rs（新建）
- moly-aitk/src/telegram_server/models.rs（新建或扩展）
- moly-aitk/Cargo.toml（添加 rusqlite 依赖）

### 禁止
- 不引入 diesel、sea-orm 等重量级 ORM
- 不在 wasm32 下编译 SQLite 代码

## 排除范围

- 全文搜索（后期再加）
- 消息加密
- 数据库备份/导出

## 验收标准

场景: 创建 Bot 并持久化
  测试: test_create_bot_persists
  当 调用 `store.create_bot("天气助手", "weather_bot")` 创建 Bot
  那么 返回 `BotInfo` 包含 id、token、name、username
  并且 重新打开数据库后 `store.get_bot_by_token(token)` 返回相同 Bot

场景: 创建 Bot 用户名重复被拒绝
  测试: test_create_bot_rejects_duplicate_username
  假设 已创建用户名为 "weather_bot" 的 Bot
  当 再次调用 `store.create_bot("另一个", "weather_bot")`
  那么 返回 `BotStoreError::DuplicateUsername`

场景: 列出所有 Bot
  测试: test_list_bots
  假设 已创建 "3" 个 Bot
  当 调用 `store.list_bots()`
  那么 返回列表长度为 "3"
  并且 按创建时间排序

场景: 更新 Bot 属性
  测试: test_update_bot
  假设 已创建 Bot token "1:moly_abc"
  当 调用 `store.update_bot(token, BotUpdate { name: Some("新名字"), .. })`
  那么 `store.get_bot_by_token(token).name` 为 "新名字"

场景: 删除 Bot 同时清除关联消息
  测试: test_delete_bot_cascades
  假设 已创建 Bot 并存储了 "5" 条消息
  当 调用 `store.delete_bot(token)`
  那么 `store.get_bot_by_token(token)` 返回 None
  并且 该 Bot 的消息记录数为 "0"

场景: 重新生成 Token
  测试: test_revoke_token
  假设 已创建 Bot 旧 token 为 "1:moly_old"
  当 调用 `store.revoke_token("1:moly_old")`
  那么 返回新 token，格式为 `{id}:{moly_hex32}`
  并且 旧 token 不再有效

场景: 存储消息并按 chat 查询
  测试: test_store_and_query_messages
  假设 Bot "1:moly_abc" 与 chat_id "1" 有 "10" 条消息
  当 调用 `store.get_messages(bot_id, chat_id, limit=5, before_id=None)`
  那么 返回最近 "5" 条消息
  并且 按时间正序排列

场景: update_id 全局递增
  测试: test_update_id_global_increment
  假设 Bot A 推入一条 update，获得 update_id "1"
  当 Bot B 推入一条 update
  那么 获得 update_id "2"

场景: offset 确认删除已消费的 updates
  测试: test_offset_acknowledges_updates
  假设 Bot "1:moly_abc" 有 "3" 条 updates，id 为 "1", "2", "3"
  当 调用 `store.get_updates(bot_id, offset=3, limit=100)`
  那么 返回 update_id >= "3" 的 updates
  并且 update_id < "3" 的 updates 可被清理

场景: schema 迁移
  测试: test_schema_migration
  假设 数据库 schema_version 为 "0"（空数据库）
  当 调用 `store.open(path)` 打开数据库
  那么 自动创建所有表
  并且 schema_version 更新为当前版本

场景: 媒体文件元信息存储
  测试: test_store_media_metadata
  当 调用 `store.store_media(file_id, file_path, mime_type, file_size)`
  那么 `store.get_media(file_id)` 返回正确的文件路径和 MIME 类型
