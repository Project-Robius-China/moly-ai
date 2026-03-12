spec: task
name: "AITK Telegram Bot API Server 核心"
tags: [stage2, aitk, telegram-api, server]
---

## 意图

在 AITK 中实现 Telegram Bot API 兼容的 HTTP 服务器核心模块。该服务器接收
teloxide 客户端（如 crew-rs）的长轮询请求，转发用户消息，并接收 Bot 回复。
这是 Stage 2 Bot-Native Messaging 的基础设施层，为 Moly 提供与任何
Telegram Bot 框架通信的能力。

模块必须通过 feature flag 和 cfg gate 保证全平台编译通过（wasm32 下禁用）。

## 约束

- 整个模块 gate 在 `cfg(not(target_arch = "wasm32"))` 下，并使用 feature
  flag `telegram-server`
- 异步原语使用 `futures::channel::mpsc` 而非 `tokio::mpsc`，长轮询等待使用
  `futures::future::select` 而非 `tokio::select!`
- 库代码禁止 `.unwrap()`，仅在不变量违反时使用 `.expect()` 并附描述
- 所有公开类型、函数、方法必须有 doc comment
- Token 格式为 `{numeric_bot_id}:{moly_random_hex}`，兼容 teloxide 的
  token 解析（teloxide 按 `:` 分割提取 bot_id）
- 同一 Bot 同时只允许一个 `getUpdates` 长轮询连接（后到的返回 409 Conflict），
  与 Telegram 真实行为一致
- 服务器状态使用 `Arc<ServerState>` 共享，per-bot 队列使用独立锁
- 单用户模式：user_id 固定为 1，每个 Bot 对应一个 chat_id（等于 bot_id）

## 已定决策

- HTTP 框架: axum（与 moly-sync 已有模式一致）
- Token 格式: `{bot_id}:{moly_hex32}`（32 位随机 hex，兼容 teloxide）
- 默认端口: 8488，端口被占用时自动选择随机端口
- 服务器在 Moly 启动时延迟启动（第一个 Bot 创建时）
- 返回 `ServerHandle` 支持 graceful shutdown（参考 moly-sync 模式）
- per-bot update queue 容量上限: 1000 条，超出后丢弃最旧的
- getUpdates timeout 范围: 0-60 秒，默认 30 秒
- 所有 API 响应遵循 Telegram 标准格式: `{"ok": true, "result": ...}`
  或 `{"ok": false, "error_code": N, "description": "..."}`
- teloxide 启动时会调用 `getWebhookInfo` 和 `deleteWebhook`，需实现为
  返回空 webhook 的 no-op

## 边界

### 允许修改
- moly-aitk/src/telegram_server/**（新建）
- moly-aitk/Cargo.toml（添加 feature flag 和依赖）
- moly-aitk/src/lib.rs（导出新模块）

### 禁止
- 不修改 moly-aitk 现有的 client 代码（OpenAI client、CrewRs client 等）
- 不添加 Makepad 依赖
- 不使用 tokio channel 原语（使用 futures channel）
- 不在 wasm32 下编译 HTTP 服务器代码

## 排除范围

- 媒体文件发送/接收（sendPhoto, sendVoice 等 → Task 3）
- SQLite 持久化存储（→ Task 2）
- BotFather 对话逻辑（→ Task 4）
- Moly UI 集成（→ Task 5）
- crew-rs base_url 修改（→ Task 6）

## 验收标准

场景: 服务器启动并监听指定端口
  测试: test_server_starts_on_configured_port
  假设 配置端口为 "8488"
  当 调用 `TelegramBotApiServer::start(config)` 启动服务器
  那么 服务器在 "http://127.0.0.1:8488" 上监听
  并且 返回 `ServerHandle` 可用于 graceful shutdown

场景: getMe 返回 Bot 信息
  测试: test_get_me_returns_bot_info
  假设 已创建名为 "天气助手" 用户名为 "weather_bot" 的 Bot，token 为 "1:moly_abc123"
  当 teloxide 调用 `GET /bot1:moly_abc123/getMe`
  那么 响应状态码为 200
  并且 响应体为:
    | 字段             | 值            |
    | ok               | true          |
    | result.id        | 1             |
    | result.is_bot    | true          |
    | result.first_name| 天气助手       |
    | result.username  | weather_bot   |

场景: 无效 token 返回 401
  测试: test_invalid_token_returns_401
  当 调用 `GET /botinvalid_token/getMe`
  那么 响应状态码为 401
  并且 响应体 `ok` 为 false
  并且 响应体 `error_code` 为 401

场景: getUpdates 长轮询 — 有消息时立即返回
  测试: test_get_updates_returns_pending_messages
  假设 已创建 Bot token "1:moly_abc123"
  并且 update queue 中有 "1" 条 offset 为 "100" 的文本消息 "你好"
  当 调用 `POST /bot1:moly_abc123/getUpdates` body 为 `{"offset": 100, "timeout": 30}`
  那么 响应在 "1" 秒内返回
  并且 result 数组长度为 "1"
  并且 result[0].update_id 为 "100"
  并且 result[0].message.text 为 "你好"

场景: getUpdates 长轮询 — 无消息时等待至超时
  测试: test_get_updates_waits_until_timeout
  假设 已创建 Bot token "1:moly_abc123"
  并且 update queue 为空
  当 调用 `POST /bot1:moly_abc123/getUpdates` body 为 `{"offset": 0, "timeout": 2}`
  那么 响应在 "2" 至 "3" 秒内返回
  并且 result 为空数组

场景: getUpdates 长轮询 — 等待期间有新消息时立即返回
  测试: test_get_updates_returns_on_new_message
  假设 已创建 Bot token "1:moly_abc123"
  并且 update queue 为空
  当 调用 `POST /bot1:moly_abc123/getUpdates` body 为 `{"offset": 0, "timeout": 30}`
  并且 在 "1" 秒后 push_update 推入一条消息
  那么 响应在 "2" 秒内返回
  并且 result 数组长度为 "1"

场景: getUpdates 并发拒绝
  测试: test_get_updates_rejects_concurrent_poller
  假设 已创建 Bot token "1:moly_abc123"
  并且 一个 getUpdates 长轮询正在进行中
  当 第二个客户端调用 `POST /bot1:moly_abc123/getUpdates`
  那么 第二个请求响应状态码为 409

场景: sendMessage 接收 Bot 回复
  测试: test_send_message_stores_and_notifies
  假设 已创建 Bot token "1:moly_abc123"
  当 调用 `POST /bot1:moly_abc123/sendMessage` body 为:
    | 字段      | 值    |
    | chat_id   | 1     |
    | text      | 你好  |
  那么 响应状态码为 200
  并且 响应体 result.message_id 为正整数
  并且 outbound channel 收到一条包含 "你好" 的消息

场景: sendMessage 带 inline keyboard
  测试: test_send_message_with_inline_keyboard
  假设 已创建 Bot token "1:moly_abc123"
  当 调用 `POST /bot1:moly_abc123/sendMessage` body 包含 reply_markup:
    | 字段                                | 值                    |
    | chat_id                             | 1                     |
    | text                                | 选择一个选项           |
    | reply_markup.inline_keyboard[0][0]  | {"text":"A","callback_data":"a"} |
  那么 响应状态码为 200
  并且 outbound channel 收到的消息包含 inline_keyboard 数据

场景: editMessageText 编辑已发送消息
  测试: test_edit_message_text
  假设 已创建 Bot token "1:moly_abc123"
  并且 Bot 已发送 message_id 为 "5" 的消息
  当 调用 `POST /bot1:moly_abc123/editMessageText` body 为:
    | 字段       | 值         |
    | chat_id    | 1          |
    | message_id | 5          |
    | text       | 更新后的文本 |
  那么 响应状态码为 200
  并且 outbound channel 收到一条 edit 类型的消息

场景: deleteMessage 删除消息
  测试: test_delete_message
  假设 已创建 Bot token "1:moly_abc123"
  并且 Bot 已发送 message_id 为 "5" 的消息
  当 调用 `POST /bot1:moly_abc123/deleteMessage` body 为:
    | 字段       | 值 |
    | chat_id    | 1  |
    | message_id | 5  |
  那么 响应状态码为 200
  并且 outbound channel 收到一条 delete 类型的消息

场景: answerCallbackQuery 应答回调
  测试: test_answer_callback_query
  假设 已创建 Bot token "1:moly_abc123"
  当 调用 `POST /bot1:moly_abc123/answerCallbackQuery` body 为:
    | 字段              | 值       |
    | callback_query_id | cq_001   |
  那么 响应状态码为 200
  并且 响应体 result 为 true

场景: getWebhookInfo 返回空 webhook（teloxide 启动兼容）
  测试: test_get_webhook_info_returns_empty
  假设 已创建 Bot token "1:moly_abc123"
  当 调用 `GET /bot1:moly_abc123/getWebhookInfo`
  那么 响应状态码为 200
  并且 result.url 为空字符串

场景: deleteWebhook no-op（teloxide 启动兼容）
  测试: test_delete_webhook_noop
  假设 已创建 Bot token "1:moly_abc123"
  当 调用 `POST /bot1:moly_abc123/deleteWebhook`
  那么 响应状态码为 200
  并且 result 为 true

场景: setMyCommands 存储并返回成功
  测试: test_set_my_commands
  假设 已创建 Bot token "1:moly_abc123"
  当 调用 `POST /bot1:moly_abc123/setMyCommands` body 包含 commands 列表
  那么 响应状态码为 200
  并且 result 为 true

场景: push_update 正确排入队列
  测试: test_push_update_enqueues_message
  假设 已创建 Bot token "1:moly_abc123"
  当 通过 `server.push_update("1:moly_abc123", update)` 推入一条文本消息
  那么 该消息可通过 getUpdates 获取

场景: 模块在 wasm32 下编译通过（不含服务器代码）
  测试: test_wasm32_compilation
  当 使用 `cargo check --target wasm32-unknown-unknown` 编译 moly-aitk
  那么 编译成功
  但是 `telegram_server` 模块未被包含

场景: 错误类型覆盖所有失败场景
  测试: test_error_types_are_defined
  当 检查 `BotApiError` 枚举定义
  那么 包含以下变体:
    | 变体             | 说明              |
    | InvalidToken     | token 无效或不存在 |
    | BotNotFound      | Bot 不存在        |
    | InvalidRequest   | 请求格式错误      |
    | ConflictPoller   | 并发轮询冲突      |
    | QueueFull        | 更新队列已满      |
    | InternalError    | 内部错误          |
