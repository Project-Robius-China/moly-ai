spec: task
name: "Stage1-Task1: OpenAI Compatible Upgrade"
tags: [stage1, openai, api-client]
---

## Intent

Upgrade AITK's OpenAI client to fully support the standard parameters of the OpenAI Chat Completions API.
The basic parameters (temperature, top_p, etc.) and advanced parameters (tool_choice, response_format,
logprobs, stream_options, etc.) have already been implemented. This task covers the completed work
and the remaining integration verification.

## Decided

- All new parameters are placed in the `OpenAiRequestOptions` struct using the `Option<T>` pattern
- Use Serde `Serialize` derive instead of hand-written `as_json()` methods
- `OpenAiToolChoice` uses a custom `Serialize` impl (mixed string/object serialization)
- `OpenAiResponseFormat` uses `#[serde(tag = "type", rename_all = "snake_case")]`
- `OpenAiStop` uses `#[serde(untagged)]`
- All `.unwrap()` calls in `OpenAiClient` replaced with `.expect("openai client lock poisoned")`
- `tool_choice` and `parallel_tool_calls` are only sent when tools is non-empty

## Boundary

### Allowed Changes
- src/clients/openai.rs
- examples/openai-standardized/**
- examples/openai-integration-test/**

### Forbidden
- Do not modify code of other clients under src/clients/
- Do not modify core trait definitions (BotClient, Message, MessageContent)
- Do not use .unwrap() in library code
- Do not introduce tokio dependencies into core library code

## Out of Scope

- WebSocket protocol support (Stage 1 Task 3)
- SSE integration with crew-rs (Stage 1 Task 2)
- Image/audio and other multimodal inputs
- Streaming usage data parsing (only sending parameters)

## Acceptance Criteria

Scenario: Standard parameter serialization
  Test: openai::tests::request_body_includes_all_standard_fields
  Given OpenAiRequestOptions configured with temperature=0.7, top_p=0.9, max_completion_tokens=256
  When build_chat_completions_request_body is called
  Then the generated JSON contains the correct temperature, top_p, max_completion_tokens fields

Scenario: tool_choice is not sent when tools is empty
  Test: openai::tests::request_body_omits_tool_choice_when_tools_empty
  Given OpenAiRequestOptions configured with tool_choice=Required
  When build_chat_completions_request_body is called and tools is empty
  Then the generated JSON does not contain the tool_choice field
  And does not contain the parallel_tool_calls field

Scenario: response_format JSON Schema serialization
  Test: openai::tests::request_body_includes_json_schema_format
  Given OpenAiRequestOptions configured with response_format=JsonSchema
  When build_chat_completions_request_body is called
  Then the generated JSON response_format.type is "json_schema"
  And contains json_schema.name, json_schema.schema, json_schema.strict fields

Scenario: Serde serialization consistency
  Test: openai::tests::request_body_includes_required_tool_choice
  Given OpenAiToolChoice::Required serialized via serde_json::to_value
  When the resulting Value is compared against expected
  Then the result is the string "required"

Scenario: New parameters logprobs and user
  Test: openai::tests::request_body_includes_logprobs_fields
  Given OpenAiRequestOptions configured with logprobs=true, top_logprobs=5
  When build_chat_completions_request_body is called
  Then the generated JSON contains logprobs=true and top_logprobs=5

Scenario: Optional parameters are not sent by default
  Test: openai::tests::request_body_omits_optional_fields_by_default
  Given default OpenAiRequestOptions
  When build_chat_completions_request_body is called
  Then the generated JSON does not contain temperature, top_p, tool_choice, response_format, logprobs, user, stream_options

Scenario: Example compiles successfully
  Test: build_openai_standardized_example
  Given examples/openai-standardized directory
  When cargo build --manifest-path examples/openai-standardized/Cargo.toml is executed
  Then compilation succeeds with no errors

Scenario: All unit tests pass
  Test: openai::tests
  Given the test module in src/clients/openai.rs
  When cargo test --lib --features api-clients -- openai::tests is executed
  Then all 13 tests pass
