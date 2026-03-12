spec: task
name: "crew-rs Telegram base_url 支持"
tags: [stage2, crew-rs, telegram, config]
---

## 意图

在 crew-rs 的 Telegram channel 配置中添加 `base_url` 字段，允许 teloxide
连接到自定义的 Telegram Bot API 服务器（如 Moly 的本地服务器）而非
api.telegram.org。这是 Moly Bot-Native Messaging 方案的 crew-rs 侧唯一改动。

## 约束

- 改动范围极小：仅修改 TelegramChannel 初始化逻辑
- 向后兼容：base_url 为 Optional，默认行为不变（连 api.telegram.org）
- 使用 teloxide 内置的 `Bot::set_api_url()` 方法设置自定义 URL
- Web dashboard 的 TelegramTab 添加可选的 API URL 输入框

## 已定决策

- 配置字段名: `base_url`（与 LLM provider 的 base_url 命名一致）
- teloxide API: 使用 `reqwest::Url::parse(base_url)` 然后
  `Bot::set_api_url(url)`
- Dashboard UI: 在 Bot Token 输入框下方添加 "API URL (Optional)"
  输入框，placeholder 为 "https://api.telegram.org"

## 边界

### 允许修改
- crates/crew-bus/src/telegram_channel.rs（TelegramChannel::new 添加参数）
- crates/crew-cli/src/commands/gateway/mod.rs（读取 base_url 配置）
- crates/crew-cli/src/config.rs（ChannelEntry settings 新增字段）
- dashboard/src/components/tabs/TelegramTab.tsx（添加 URL 输入框）

### 禁止
- 不修改 TelegramChannel 的消息处理逻辑
- 不修改其他 channel 类型
- 不破坏无 base_url 时的默认行为

## 排除范围

- TELOXIDE_TELEGRAM_API_URL 环境变量支持（teloxide 已内置）
- URL 有效性验证（超出 URL 解析之外的验证）
- 自动发现 Moly 服务器

## 验收标准

场景: 不配置 base_url 时行为不变
  测试: test_default_telegram_url
  假设 channel 配置不包含 "base_url" 字段
  当 创建 TelegramChannel
  那么 teloxide Bot 使用默认的 "https://api.telegram.org" URL

场景: 配置 base_url 后连接自定义服务器
  测试: test_custom_base_url
  假设 channel 配置 base_url 为 "http://localhost:8488"
  当 创建 TelegramChannel 并调用 getMe
  那么 HTTP 请求发送到 "http://localhost:8488/bot{token}/getMe"

场景: 无效 base_url 返回错误
  测试: test_invalid_base_url_returns_error
  假设 channel 配置 base_url 为 "not-a-url"
  当 创建 TelegramChannel
  那么 返回配置错误，包含 URL 解析失败信息

场景: Dashboard 显示 API URL 输入框
  测试: test_dashboard_shows_api_url_input
  当 查看 Dashboard 的 Telegram 配置页
  那么 显示 "API URL" 可选输入框
  并且 placeholder 为 "https://api.telegram.org"
