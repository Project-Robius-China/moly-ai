spec: task
name: "Stage1-Task1: OpenAI Compatible 升级"
tags: [stage1, openai, api-client]
---

## 意图

升级 aitk 的 OpenAI 客户端，使其完整支持 OpenAI Chat Completions API 的标准参数。
当前已完成基础参数（temperature, top_p 等）和高级参数（tool_choice, response_format,
logprobs, stream_options 等）的实现。此 Task 覆盖已完成的工作和剩余的集成验证。

## 已定决策

- 所有新参数放在 `OpenAiRequestOptions` 结构体中，使用 `Option<T>` 模式
- 使用 Serde `Serialize` derive 替代手写 `as_json()` 方法
- `OpenAiToolChoice` 使用自定义 `Serialize` impl（混合 string/object 序列化）
- `OpenAiResponseFormat` 使用 `#[serde(tag = "type", rename_all = "snake_case")]`
- `OpenAiStop` 使用 `#[serde(untagged)]`
- `OpenAiClient` 中所有 `.unwrap()` 替换为 `.expect("openai client lock poisoned")`
- `tool_choice` 和 `parallel_tool_calls` 仅在 tools 非空时发送

## 边界

### Allowed Changes
- src/clients/openai.rs
- examples/openai-standardized/**
- examples/openai-integration-test/**

### Forbidden
- 不要修改 src/clients/ 下其他客户端的代码
- 不要修改核心 trait 定义（BotClient, Message, MessageContent）
- 不要在库代码中使用 .unwrap()
- 不要引入 tokio 依赖到核心库代码

## 排除范围

- WebSocket 协议支持（Stage 1 Task 3）
- SSE 对接 crew-rs（Stage 1 Task 2）
- 图片/音频等多模态输入
- Streaming 的 usage 数据解析（仅发送参数）

## 完成条件

Scenario: 标准参数序列化
  Test: openai::tests::request_body_includes_all_standard_fields
  Given OpenAiRequestOptions 配置了 temperature=0.7, top_p=0.9, max_completion_tokens=256
  When 调用 build_chat_completions_request_body
  Then 生成的 JSON 包含正确的 temperature, top_p, max_completion_tokens 字段

Scenario: tool_choice 在 tools 为空时不发送
  Test: openai::tests::request_body_omits_tool_choice_when_tools_empty
  Given OpenAiRequestOptions 配置了 tool_choice=Required
  When 调用 build_chat_completions_request_body 且 tools 为空
  Then 生成的 JSON 不包含 tool_choice 字段
  And 不包含 parallel_tool_calls 字段

Scenario: response_format JSON Schema 序列化
  Test: openai::tests::request_body_includes_json_schema_format
  Given OpenAiRequestOptions 配置了 response_format=JsonSchema
  When 调用 build_chat_completions_request_body
  Then 生成的 JSON response_format.type 为 "json_schema"
  And 包含 json_schema.name, json_schema.schema, json_schema.strict 字段

Scenario: Serde 序列化一致性
  Test: openai::tests::request_body_includes_required_tool_choice
  Given OpenAiToolChoice::Required 通过 serde_json::to_value 序列化
  When 得到的 Value 与预期对比
  Then 结果为 "required" 字符串

Scenario: 新增参数 logprobs 和 user
  Test: openai::tests::request_body_includes_logprobs_fields
  Given OpenAiRequestOptions 配置了 logprobs=true, top_logprobs=5
  When 调用 build_chat_completions_request_body
  Then 生成的 JSON 包含 logprobs=true 和 top_logprobs=5

Scenario: 可选参数默认不发送
  Test: openai::tests::request_body_omits_optional_fields_by_default
  Given 默认的 OpenAiRequestOptions
  When 调用 build_chat_completions_request_body
  Then 生成的 JSON 不包含 temperature, top_p, tool_choice, response_format, logprobs, user, stream_options

Scenario: Example 编译通过
  Test: build_openai_standardized_example
  Given examples/openai-standardized 目录
  When 执行 cargo build --manifest-path examples/openai-standardized/Cargo.toml
  Then 编译成功，无错误

Scenario: 所有单元测试通过
  Test: openai::tests
  Given src/clients/openai.rs 中的测试模块
  When 执行 cargo test --lib --features api-clients -- openai::tests
  Then 全部 13 个测试通过
