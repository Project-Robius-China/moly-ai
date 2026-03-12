spec: task
name: "Moly Telegram Server 集成启动"
tags: [stage2, moly, integration, telegram-server]
---

## 意图

将 AITK 的 telegram-server 模块集成到 Moly App 中：更新 AITK 依赖到
最新版本（包含 telegram-server feature），在 Moly 启动时自动启动 Telegram
Bot API 服务器，将 ServerState 存入 App Store，并启动 outbound 事件消费
循环。

这是 BotFather（task4）和 Bot Chat Integration（task5）的前置任务。
完成后 Moly 即具备接收 crew-rs teloxide 连接的能力。

## 约束

- AITK 依赖使用 git rev 指向 PR #2 合并后的 mainline commit
- telegram-server feature 仅在 native 平台启用，wasm32 排除
- 服务器监听 127.0.0.1，端口可配（默认 8488）
- ServerState 存储在 Store 中，供后续 BotFather/Chat 使用
- outbound 事件循环异步运行，日志级别记录事件（暂不做 UI 展示）
- 使用 AITK 的 spawn() 函数（跨平台异步），不直接用 tokio::spawn

## 已定决策

- AITK 依赖在 moly-kit/Cargo.toml 中更新 rev + 添加 "telegram-server" feature
- moly-kit 的 telegram-server feature 通过 cfg gate 传递
- ServerState 以 Option<Arc<ServerState>> 存入 Store（wasm 下为 None）
- ServerHandle 由 Store 持有，Drop 时自动 shutdown 服务器
- 端口号写入 Preferences（默认 8488），供 BotFather 在创建 Bot 时告知用户
- outbound_rx 在单独 spawn 的 task 中消费，暂时只 log::info 打印

## 边界

### 允许修改
- moly-kit/Cargo.toml（更新 aitk 依赖）
- src/data/store.rs（添加 ServerState 字段和初始化）
- src/data/preferences.rs（添加 bot_server_port 配置项）
- src/app.rs（启动时初始化服务器）
- Cargo.toml（workspace 层面，如需要）

### 禁止
- 不修改 AITK 库代码
- 不修改现有 Provider/Chat 逻辑
- 不添加任何 Bot UI（那是 task4/task5 的事）

## 排除范围

- BotFather 对话逻辑（→ task4）
- Bot 聊天 UI 集成（→ task5）
- 自动创建 Bot（→ task4）
- Media API 端点（→ task3-media）

## 验收标准

场景: Moly 启动后 telegram server 自动运行
  测试: test_server_starts_on_app_init
  当 Moly App 完成 Store 初始化
  那么 ServerState 不为 None
  并且 服务器在 127.0.0.1:8488 监听

场景: curl getMe 返回 404（无 Bot）
  测试: test_server_responds_to_requests
  假设 Moly 已启动且服务器运行中
  当 curl 请求 http://127.0.0.1:8488/botinvalid/getMe
  那么 返回 HTTP 200，body 中 ok=false

场景: 创建 Bot 后 getMe 返回正确信息
  测试: test_create_bot_and_get_me
  假设 Moly 已启动且服务器运行中
  当 通过 ServerState.store.create_bot("Test", "test_bot") 创建 Bot
  并且 curl 请求 http://127.0.0.1:8488/bot{token}/getMe
  那么 返回 HTTP 200，body 中 ok=true，username="test_bot"

场景: wasm32 平台下服务器不启动
  测试: test_no_server_on_wasm
  假设 编译目标为 wasm32
  当 Store 初始化完成
  那么 ServerState 为 None
  并且 无编译错误

场景: App 退出时服务器自动关闭
  测试: test_server_shuts_down_on_drop
  假设 服务器已启动
  当 Store 被 Drop
  那么 ServerHandle 触发 shutdown
  并且 端口释放
