spec: task
name: "Stage1-Task2: SSE 对接 crew-rs"
tags: [stage1, sse, crew-rs]
---

## 意图

通过 SSE（Server-Sent Events）对接 crew-rs gateway。moly-ai 已支持 SSE 的 bot 接入，
crew-rs gateway 也已支持 SSE。本任务是连通两端，使 moly-ai 能通过 SSE 与 crew-rs 进行流式对话。

## 已定决策

- 复用现有 OpenAI SSE 流式解析（parse_sse）
- crew-rs gateway 的 SSE endpoint 兼容 OpenAI 格式
- 不引入新的 HTTP 客户端库，使用现有 reqwest

## 边界

### Allowed Changes
- src/clients/**
- examples/crew-rs-sse/**

### Forbidden
- 不要修改核心 trait（BotClient）的签名
- 不要引入 tokio 运行时依赖到库代码

## 排除范围

- WebSocket 双工通信（Task 3）
- 工具审批 UI（Task 4）
- 认证/握手流程的复杂实现

## 完成条件

Scenario: SSE 流式连接 crew-rs
  Test: test_sse_crew_rs_connection
  Given crew-rs gateway 在本地运行
  When moly-ai 通过 SSE 发送聊天请求
  Then 接收到流式响应 chunks
  And 最终组装成完整的 MessageContent

Scenario: SSE 错误处理
  Test: test_sse_connection_error
  Given crew-rs gateway 不可达
  When moly-ai 尝试 SSE 连接
  Then 返回 ClientError::Network 错误
