spec: task
name: "Moly BotFather 对话管理器"
tags: [stage2, moly, botfather, ui]
---

## 意图

在 Moly App 层实现 BotFather 对话管理器——一个内置的特殊 Bot，用户通过
发送 /command 命令来创建、编辑、管理其他 Bot。BotFather 复刻 Telegram
BotFather 的交互模式：对话式向导、内联按钮导航、即时反馈。

BotFather 的对话逻辑完全在本地运行（不经过网络），调用 AITK 的 Bot Store
CRUD 接口操作数据。

## 约束

- BotFather 命令解析和状态机在 Moly App 层实现，不放在 AITK
- BotFather 是不可删除、不可重命名的内置 Bot，启动时自动出现在聊天列表首位
- 命令解析支持带 `/` 前缀的命令和自然语言输入（未匹配命令时提示可用命令）
- 所有操作立即生效并持久化到 SQLite
- 创建 Bot 后自动在聊天列表中显示新 Bot

## 已定决策

- 对话状态机使用 enum 表示当前状态（Idle、AwaitingBotName、AwaitingUsername 等）
- /mybots 使用内联按钮列表展示，点击后进入 Bot 管理子菜单
- username 校验规则: 3-32 字符，仅小写字母/数字/下划线，必须以 `_bot` 或 `bot` 结尾
- BotFather 回复中包含操作指引文字（如何粘贴 token 到 crew-rs）
- Bot 头像使用默认生成图标（基于名称首字母），/setuserpic 允许自定义

## 边界

### 允许修改
- src/bot_manager/**（新建）
- src/data/bots.rs（新建，Bot 数据类型供 App 使用）
- src/chat/ 相关文件（添加 BotFather 聊天视图）
- src/app_state.rs（添加 BotManager 状态）

### 禁止
- 不修改 AITK 库代码（只调用其接口）
- 不修改现有 Provider 管理流程（并行共存）
- 不添加网络请求（BotFather 纯本地）

## 排除范围

- /setcommands（Bot 命令菜单配置 → Phase B）
- Bot 头像图片处理/裁剪
- BotFather 国际化（先只支持中文）

## 验收标准

场景: /start 显示欢迎消息和命令列表
  测试: test_botfather_start_command
  当 用户向 BotFather 发送 "/start"
  那么 BotFather 回复包含欢迎信息
  并且 回复包含可用命令列表（/newbot, /mybots, /setname 等）

场景: /newbot 完整创建流程
  测试: test_botfather_newbot_flow
  当 用户向 BotFather 发送 "/newbot"
  那么 BotFather 回复 "请给它起个名字"
  当 用户输入 "天气助手"
  那么 BotFather 回复 "请起一个用户名（必须以 bot 结尾）"
  当 用户输入 "weather_bot"
  那么 BotFather 回复包含 "已创建" 和 token
  并且 新 Bot 出现在聊天列表中

场景: /newbot 用户名不合法被拒绝
  测试: test_botfather_newbot_invalid_username
  假设 BotFather 正在等待用户输入用户名
  当 用户输入 "invalid name with spaces"
  那么 BotFather 回复用户名格式要求
  并且 对话状态仍为等待用户名输入

场景: /newbot 用户名已存在被拒绝
  测试: test_botfather_newbot_duplicate_username
  假设 已存在用户名为 "weather_bot" 的 Bot
  并且 BotFather 正在等待用户输入用户名
  当 用户输入 "weather_bot"
  那么 BotFather 回复用户名已被使用
  并且 对话状态仍为等待用户名输入

场景: /mybots 列出 Bot 并支持按钮选择
  测试: test_botfather_mybots
  假设 已创建 "2" 个 Bot: "天气助手" 和 "代码助手"
  当 用户向 BotFather 发送 "/mybots"
  那么 BotFather 回复包含 "2" 个内联按钮
  当 用户点击 "天气助手" 按钮
  那么 BotFather 显示管理子菜单（编辑名称、查看Token、删除等按钮）

场景: /mybots 无 Bot 时提示
  测试: test_botfather_mybots_empty
  假设 未创建任何 Bot
  当 用户向 BotFather 发送 "/mybots"
  那么 BotFather 回复 "还没有创建任何 Bot" 并提示使用 /newbot

场景: /token 显示 token 和使用说明
  测试: test_botfather_token_command
  假设 已选择管理 "天气助手" Bot
  当 用户点击 "查看 Token" 按钮
  那么 BotFather 回复包含该 Bot 的 token
  并且 回复包含 crew-rs 配置说明（API URL 和 token 粘贴步骤）

场景: /revoke 重新生成 token
  测试: test_botfather_revoke_token
  假设 已选择管理 "天气助手" Bot，旧 token 为 "1:moly_old"
  当 用户点击 "重置 Token" 按钮
  那么 BotFather 确认 token 已重置
  并且 显示新 token，与旧 token 不同

场景: /setname 修改 Bot 名称
  测试: test_botfather_setname
  假设 已选择管理 "天气助手" Bot
  当 用户点击 "编辑名称"
  那么 BotFather 回复 "请输入新名称"
  当 用户输入 "天气预报大师"
  那么 BotFather 确认名称已更新
  并且 聊天列表中的 Bot 名称更新为 "天气预报大师"

场景: /deletebot 删除 Bot 需确认
  测试: test_botfather_deletebot_confirms
  假设 已选择管理 "天气助手" Bot
  当 用户点击 "删除 Bot"
  那么 BotFather 回复确认提示（"确定要删除吗？"）
  当 用户确认删除
  那么 Bot 从聊天列表中移除
  并且 关联的消息历史被清理

场景: 未识别命令提示帮助
  测试: test_botfather_unknown_command
  当 用户向 BotFather 发送 "随便说点什么"
  那么 BotFather 回复提示使用 /start 查看可用命令
