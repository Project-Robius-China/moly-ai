# OpenAI Compatible Improvements Design

## Context

Branch `feat_openai_compatible_stage1_task1` added `OpenAiRequestOptions` with standard
Chat Completions parameters (tool_choice, response_format, temperature, etc.).
Code review identified several issues and improvement opportunities.

## Changes

### 1. Bug Fixes

- Remove extra blank line at line 847-848 in `openai.rs`.
- Replace `.unwrap()` with `.expect("openai client lock poisoned")` in all `set_*` methods.
- Skip `tool_choice` and `parallel_tool_calls` in request body when `tools` slice is empty.

### 2. Serde Refactor

Replace hand-written `as_json()` methods with `#[derive(Serialize)]`:

- `OpenAiToolChoice`: custom `Serialize` impl (string variants + struct variant).
- `OpenAiResponseFormat`: `#[serde(tag = "type", rename_all = "snake_case")]`.
- `OpenAiStop`: `#[serde(untagged)]`.
- `OpenAiJsonSchemaResponseFormat`: `#[derive(Serialize)]` with `#[serde(rename)]`.
- Update `build_chat_completions_request_body` to use `serde_json::to_value()`.

### 3. New Parameters

Add commonly used parameters missing from the current implementation:

| Parameter | Type | Range | Purpose |
|-----------|------|-------|---------|
| `logprobs` | `Option<bool>` | - | Return log probabilities |
| `top_logprobs` | `Option<u8>` | 0-20 | Number of top logprobs per token |
| `user` | `Option<String>` | - | End-user identifier for abuse monitoring |
| `stream_options` | `Option<OpenAiStreamOptions>` | - | Stream options (e.g. `include_usage`) |

### 4. Documentation

Add valid range annotations to doc comments for numeric parameters:

- `temperature`: `0.0..=2.0`
- `top_p`: `0.0..=1.0`
- `presence_penalty` / `frequency_penalty`: `-2.0..=2.0`
- `stop`: up to 4 sequences
- `top_logprobs`: `0..=20`

### 5. Tests

- Verify `tool_choice` absent when tools empty.
- Verify Serde serialization matches expected JSON output.
- Verify new parameters serialize correctly.
- Verify default options produce minimal request body.
