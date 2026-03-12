spec: task
name: "Stage1-Task3: WebSocket Gateway"
status: cancelled
tags: [stage1, websocket, crew-rs]
---

## 意图

实现 WebSocket 客户端连接 crew-rs WebSocket Gateway，支持双工通信。
服务端可主动推送状态，客户端可发送聊天消息和接收流式响应。
moly-ai 已有 openclaw 的 WebSocket 支持可作为参考。

## 已定决策

- 使用 futures 的 channel 而非 tokio channel（跨平台兼容）
- WebSocket 协议层使用 tungstenite 或类似库
- 消息格式为 JSON，兼容 crew-rs 的帧协议
- 认证通过 WebSocket 握手 header 传递 API key

## 边界

### Allowed Changes
- src/clients/**
- src/utils/**

### Forbidden
- 不要修改现有 OpenAI 客户端的行为
- 不要在库代码中使用 tokio::spawn
- 不要硬编码 WebSocket 地址

## 排除范围

- 工具审批 UI（Task 4 独立实现）
- 记忆帧协议（Stage 3）
- 断线重连的高级策略

## 完成条件

Scenario: WebSocket 握手和认证
  Test: test_ws_handshake_with_auth
  Given crew-rs WebSocket gateway 地址和 API key
  When 客户端发起 WebSocket 连接
  Then 握手成功并建立连接

Scenario: 流式聊天消息收发
  Test: test_ws_chat_streaming
  Given 已建立 WebSocket 连接
  When 发送聊天消息
  Then 接收到多个流式 chunk
  And 最终组装成完整 MessageContent

Scenario: 服务端主动推送
  Test: test_ws_server_push
  Given 已建立 WebSocket 连接
  When 服务端推送状态更新帧
  Then 客户端正确解析并回调

Scenario: 连接错误处理
  Test: test_ws_connection_error
  Given 无效的 WebSocket 地址
  When 客户端尝试连接
  Then 返回有意义的错误信息
