spec: task
name: "Moly Bot 聊天 UI 集成"
tags: [stage2, moly, ui, chat]
---

## 意图

将 AITK Telegram Bot API Server 与 Moly 的聊天 UI 集成，使用户可以在
Moly 中直接与 Bot 进行对话。用户发送的消息通过 push_update 传给 crew-rs，
crew-rs 的回复通过 recv_outbound 显示在 Moly 聊天界面。支持文本消息、
inline keyboard 按钮交互、媒体消息渲染和 Bot 连接状态显示。

## 约束

- 复用现有 MolyKit Chat widget 组件，不重写聊天 UI
- Bot 聊天与 BotFather 聊天使用相同的 Chat 组件，通过消息路由区分
- 连接状态通过 getUpdates 请求频率检测：2 分钟无请求标记为离线
- inline keyboard 按钮点击发送 callback_query 类型的 update
- 媒体消息（图片、文档等）需在聊天中正确渲染

## 已定决策

- Bot 列表在聊天侧栏中展示，BotFather 固定首位，其余按最后活跃时间排序
- 连接状态指示器: 绿点=在线、灰点=离线，显示在 Bot 头像旁
- inline keyboard 渲染为按钮行，点击后发送 callback_data
- 用户发送的消息构造为 Telegram Update 对象，包含 Message 结构

## 边界

### 允许修改
- src/bot_manager/**（扩展）
- src/chat/**（添加 Bot 聊天路由和消息适配）
- src/data/bots.rs（扩展）
- src/app_state.rs（添加 Bot 连接状态追踪）

### 禁止
- 不重写 MolyKit Chat widget
- 不修改 AITK 库代码（只调用接口）
- 不修改现有 Provider chat 行为

## 排除范围

- 消息搜索功能
- 打字指示器（typing indicator）动画
- Bot 头像自定义渲染

## 验收标准

场景: Bot 出现在聊天列表中
  测试: test_bot_appears_in_chat_list
  假设 通过 BotFather 创建了 "天气助手" Bot
  当 查看聊天侧栏
  那么 "天气助手" 出现在列表中，BotFather 之下

场景: 发送文本消息到 Bot
  测试: test_send_text_to_bot
  假设 用户进入 "天气助手" 的聊天界面
  当 用户输入 "今天天气怎么样" 并发送
  那么 消息显示在聊天界面右侧（用户消息）
  并且 AITK 的 update queue 中出现该消息的 Update 对象

场景: 接收 Bot 文本回复
  测试: test_receive_bot_text_reply
  假设 crew-rs 通过 sendMessage 发送了 "今天晴天，25°C"
  当 Moly 通过 recv_outbound 接收到该消息
  那么 消息显示在聊天界面左侧（Bot 消息）

场景: 渲染 inline keyboard 按钮
  测试: test_render_inline_keyboard
  假设 crew-rs 发送的消息包含 inline_keyboard:
    | row | text    | callback_data |
    | 0   | 详细天气 | weather_detail |
    | 0   | 未来3天  | weather_3day   |
  当 Moly 显示该消息
  那么 消息下方渲染 "2" 个按钮

场景: 点击 inline keyboard 按钮发送 callback
  测试: test_inline_keyboard_click_sends_callback
  假设 消息下方有 callback_data 为 "weather_detail" 的按钮
  当 用户点击该按钮
  那么 AITK 的 update queue 中出现一条 callback_query 类型的 Update
  并且 callback_query.data 为 "weather_detail"

场景: Bot 连接状态 — 在线
  测试: test_bot_online_status
  假设 crew-rs 在过去 "30" 秒内调用过 getUpdates
  当 查看聊天列表中 "天气助手" 的状态
  那么 显示绿色在线指示器

场景: Bot 连接状态 — 离线
  测试: test_bot_offline_status
  假设 crew-rs 超过 "2" 分钟未调用 getUpdates
  当 查看聊天列表中 "天气助手" 的状态
  那么 显示灰色离线指示器

场景: 接收媒体消息并渲染
  测试: test_receive_media_message
  假设 crew-rs 通过 sendPhoto 发送了一张图片带 caption "天气图"
  当 Moly 接收并显示该消息
  那么 聊天界面显示图片预览
  并且 图片下方显示 caption "天气图"

场景: Bot 消息编辑实时更新
  测试: test_message_edit_updates_ui
  假设 Bot 已发送 message_id "5" 的消息 "正在处理..."
  当 crew-rs 调用 editMessageText 将内容改为 "处理完成！"
  那么 聊天界面中 message_id "5" 的消息文本更新为 "处理完成！"

场景: Bot 消息删除从界面移除
  测试: test_message_delete_removes_from_ui
  假设 Bot 已发送 message_id "5" 的消息
  当 crew-rs 调用 deleteMessage 删除该消息
  那么 聊天界面中不再显示该消息

场景: 应用重启后恢复聊天历史
  测试: test_restore_chat_history_on_restart
  假设 Bot "天气助手" 有 "10" 条历史消息存在 SQLite 中
  当 Moly 应用重启并打开 "天气助手" 聊天
  那么 显示 "10" 条历史消息
