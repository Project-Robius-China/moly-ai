# OpenAI Compatible Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix review issues, refactor serialization to Serde, add missing parameters, improve docs and tests.

**Architecture:** All changes in `src/clients/openai.rs`. Serde `Serialize` derives replace hand-written `as_json()`. New parameters follow existing `Option<T>` pattern. Tests validate JSON output.

**Tech Stack:** Rust, serde, serde_json

---

### Task 1: Bug Fixes — extra blank line, `.unwrap()` → `.expect()`

**Files:**
- Modify: `src/clients/openai.rs:847-848` (extra blank line)
- Modify: `src/clients/openai.rs:709-776` (all `.unwrap()` in setters/getters)

**Step 1: Remove extra blank line**

In `src/clients/openai.rs`, lines 847-848 have two consecutive blank lines after `build_chat_completions_request_body` call. Remove one.

Before:
```rust
            );


            let request = inner
```

After:
```rust
            );

            let request = inner
```

**Step 2: Replace all `.unwrap()` with `.expect()` in `OpenAiClient` methods**

Replace every occurrence of `.write().unwrap()` and `.read().unwrap()` inside the `OpenAiClient` impl blocks with `.expect("openai client lock poisoned")`.

Affected lines (approximate): 693, 696, 709, 720, 725, 730, 735, 740, 745, 750, 755, 760, 765, 770, 775, 781, 813.

**Step 3: Run tests to verify nothing breaks**

Run: `cargo test --lib --features api-clients -- openai::tests`
Expected: All 8 tests pass.

**Step 4: Commit**

```bash
git add src/clients/openai.rs
git commit -m "fix(openai): remove extra blank line and replace .unwrap() with .expect()"
```

---

### Task 2: Bug Fix — skip `tool_choice`/`parallel_tool_calls` when tools empty

**Files:**
- Modify: `src/clients/openai.rs:607-608,631-633` (`build_chat_completions_request_body`)
- Modify: `src/clients/openai.rs` (tests section)

**Step 1: Write the failing test**

Add to `mod tests`:

```rust
#[test]
fn request_body_omits_tool_choice_when_tools_empty() {
    let options = OpenAiRequestOptions::default()
        .with_tool_choice(OpenAiToolChoice::Required)
        .with_parallel_tool_calls(true);
    let body = build_chat_completions_request_body(
        "gpt-test",
        &[sample_user_message()],
        &[],
        &options,
    );

    assert!(
        body.get("tool_choice").is_none(),
        "tool_choice should not be sent when tools is empty"
    );
    assert!(
        body.get("parallel_tool_calls").is_none(),
        "parallel_tool_calls should not be sent when tools is empty"
    );
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test --lib --features api-clients -- openai::tests::request_body_omits_tool_choice_when_tools_empty`
Expected: FAIL — `tool_choice` is currently sent regardless.

**Step 3: Fix `build_chat_completions_request_body`**

Guard `tool_choice` and `parallel_tool_calls` behind `!tools.is_empty()`:

```rust
if !tools.is_empty() {
    json["tools"] = serde_json::json!(tools);

    if let Some(tool_choice) = &options.tool_choice {
        json["tool_choice"] = tool_choice.as_json();
    }

    if let Some(parallel_tool_calls) = options.parallel_tool_calls {
        json["parallel_tool_calls"] = serde_json::json!(parallel_tool_calls);
    }
}
```

Remove the standalone `tool_choice` and `parallel_tool_calls` blocks that are outside the `if !tools.is_empty()` guard.

**Step 4: Update the existing test `request_body_includes_required_tool_choice`**

This test currently passes empty tools but expects `tool_choice` to appear. It needs a dummy tool:

```rust
fn sample_function_tool() -> FunctionTool {
    FunctionTool {
        r#type: "function".to_string(),
        function: Function {
            name: "test_fn".to_string(),
            description: "A test function".to_string(),
            parameters: serde_json::json!({"type": "object"}),
            strict: false,
        },
    }
}
```

Update the test to pass `&[sample_function_tool()]` instead of `&[]`.

Also update `request_body_includes_parallel_tool_calls_and_seed` — the `parallel_tool_calls` assertion needs a tool present. Split `seed` check into a separate assertion that still uses empty tools.

**Step 5: Run all tests**

Run: `cargo test --lib --features api-clients -- openai::tests`
Expected: All tests pass (8 existing + 1 new = 9).

**Step 6: Commit**

```bash
git add src/clients/openai.rs
git commit -m "fix(openai): skip tool_choice and parallel_tool_calls when tools is empty"
```

---

### Task 3: Serde Refactor — replace `as_json()` with `Serialize` derives

**Files:**
- Modify: `src/clients/openai.rs:411-527` (type definitions)
- Modify: `src/clients/openai.rs:591-648` (`build_chat_completions_request_body`)

**Step 1: Refactor `OpenAiStop`**

Replace:
```rust
pub enum OpenAiStop {
    Single(String),
    Multiple(Vec<String>),
}

impl OpenAiStop {
    fn as_json(&self) -> serde_json::Value { ... }
}
```

With:
```rust
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum OpenAiStop {
    /// A single stop sequence.
    Single(String),
    /// Multiple stop sequences (up to 4).
    Multiple(Vec<String>),
}
```

**Step 2: Refactor `OpenAiToolChoice`**

This needs a custom `Serialize` impl because the string variants serialize as plain strings
while `Function` serializes as an object.

Replace:
```rust
pub enum OpenAiToolChoice { ... }
impl OpenAiToolChoice { fn as_json(...) { ... } }
```

With:
```rust
#[derive(Clone, Debug, PartialEq)]
pub enum OpenAiToolChoice {
    /// The model must not call tools.
    None,
    /// Let the model decide whether to call tools.
    Auto,
    /// The model must call one or more tools.
    Required,
    /// Force a specific tool by function name.
    Function { name: String },
}

impl Serialize for OpenAiToolChoice {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            OpenAiToolChoice::None => serializer.serialize_str("none"),
            OpenAiToolChoice::Auto => serializer.serialize_str("auto"),
            OpenAiToolChoice::Required => serializer.serialize_str("required"),
            OpenAiToolChoice::Function { name } => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("type", "function")?;
                map.serialize_entry("function", &serde_json::json!({"name": name}))?;
                map.end()
            }
        }
    }
}
```

**Step 3: Refactor `OpenAiResponseFormat` and `OpenAiJsonSchemaResponseFormat`**

Replace both types and `as_json()`:

```rust
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OpenAiResponseFormat {
    /// Default text output.
    Text,
    /// JSON object output mode.
    JsonObject,
    /// Structured output using JSON Schema.
    JsonSchema {
        /// The JSON schema configuration.
        json_schema: OpenAiJsonSchemaResponseFormat,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct OpenAiJsonSchemaResponseFormat {
    /// Schema identifier expected by OpenAI-compatible APIs.
    pub name: String,
    /// JSON Schema document.
    pub schema: serde_json::Value,
    /// Whether the model output should strictly follow the schema.
    pub strict: bool,
}
```

Update `OpenAiResponseFormat::json_schema` constructor:
```rust
pub fn json_schema(name: String, schema: serde_json::Value, strict: bool) -> Self {
    OpenAiResponseFormat::JsonSchema {
        json_schema: OpenAiJsonSchemaResponseFormat { name, schema, strict },
    }
}
```

**Step 4: Refactor `OpenAiStreamOptions` (new type, needed for Task 4)**

```rust
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct OpenAiStreamOptions {
    /// Whether to include token usage data in stream responses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}
```

**Step 5: Update `build_chat_completions_request_body`**

Replace all `as_json()` calls with `serde_json::to_value()`:

```rust
if let Some(tool_choice) = &options.tool_choice {
    json["tool_choice"] = serde_json::to_value(tool_choice)
        .expect("OpenAiToolChoice serialization cannot fail");
}

if let Some(response_format) = &options.response_format {
    json["response_format"] = serde_json::to_value(response_format)
        .expect("OpenAiResponseFormat serialization cannot fail");
}

// ... similar for stop
if let Some(stop) = &options.stop {
    json["stop"] = serde_json::to_value(stop)
        .expect("OpenAiStop serialization cannot fail");
}
```

**Step 6: Run tests**

Run: `cargo test --lib --features api-clients -- openai::tests`
Expected: All 9 tests pass — serialization output unchanged.

**Step 7: Commit**

```bash
git add src/clients/openai.rs
git commit -m "refactor(openai): replace as_json() with Serde Serialize derives"
```

---

### Task 4: Add Missing Parameters — logprobs, top_logprobs, user, stream_options

**Files:**
- Modify: `src/clients/openai.rs` (`OpenAiRequestOptions`, setters, builder methods, request body builder)

**Step 1: Write failing tests for new parameters**

Add to `mod tests`:

```rust
#[test]
fn request_body_includes_logprobs_fields() {
    let options = OpenAiRequestOptions::default()
        .with_logprobs(true)
        .with_top_logprobs(5);
    let body = build_chat_completions_request_body(
        "gpt-test",
        &[sample_user_message()],
        &[],
        &options,
    );

    assert_eq!(body["logprobs"], true);
    assert_eq!(body["top_logprobs"], 5);
}

#[test]
fn request_body_includes_user_field() {
    let options = OpenAiRequestOptions::default()
        .with_user("user-123".to_string());
    let body = build_chat_completions_request_body(
        "gpt-test",
        &[sample_user_message()],
        &[],
        &options,
    );

    assert_eq!(body["user"], "user-123");
}

#[test]
fn request_body_includes_stream_options() {
    let options = OpenAiRequestOptions::default()
        .with_stream_options(OpenAiStreamOptions {
            include_usage: Some(true),
        });
    let body = build_chat_completions_request_body(
        "gpt-test",
        &[sample_user_message()],
        &[],
        &options,
    );

    assert_eq!(body["stream_options"]["include_usage"], true);
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test --lib --features api-clients -- openai::tests`
Expected: 3 new tests fail (methods don't exist yet).

**Step 3: Add fields to `OpenAiRequestOptions`**

```rust
/// Whether to return log probabilities of output tokens.
pub logprobs: Option<bool>,
/// Number of most likely tokens to return at each position (`0..=20`).
/// Requires `logprobs` to be `true`.
pub top_logprobs: Option<u8>,
/// End-user identifier for OpenAI abuse monitoring.
pub user: Option<String>,
/// Stream-specific options such as `include_usage`.
pub stream_options: Option<OpenAiStreamOptions>,
```

**Step 4: Add builder methods to `OpenAiRequestOptions`**

```rust
/// Returns options with `logprobs` configured.
pub fn with_logprobs(mut self, logprobs: bool) -> Self {
    self.logprobs = Some(logprobs);
    self
}

/// Returns options with `top_logprobs` configured (`0..=20`).
pub fn with_top_logprobs(mut self, top_logprobs: u8) -> Self {
    self.top_logprobs = Some(top_logprobs);
    self
}

/// Returns options with `user` configured.
pub fn with_user(mut self, user: String) -> Self {
    self.user = Some(user);
    self
}

/// Returns options with `stream_options` configured.
pub fn with_stream_options(mut self, stream_options: OpenAiStreamOptions) -> Self {
    self.stream_options = Some(stream_options);
    self
}
```

**Step 5: Add setter methods to `OpenAiClient`**

```rust
/// Sets the `logprobs` option for future chat completion requests.
pub fn set_logprobs(&mut self, logprobs: Option<bool>) {
    self.0.write().expect("openai client lock poisoned").request_options.logprobs = logprobs;
}

/// Sets the `top_logprobs` option (`0..=20`) for future chat completion requests.
/// Requires `logprobs` to be `Some(true)`.
pub fn set_top_logprobs(&mut self, top_logprobs: Option<u8>) {
    self.0.write().expect("openai client lock poisoned").request_options.top_logprobs = top_logprobs;
}

/// Sets the `user` option for future chat completion requests.
pub fn set_user(&mut self, user: Option<String>) {
    self.0.write().expect("openai client lock poisoned").request_options.user = user;
}

/// Sets the `stream_options` for future chat completion requests.
pub fn set_stream_options(&mut self, stream_options: Option<OpenAiStreamOptions>) {
    self.0.write().expect("openai client lock poisoned").request_options.stream_options = stream_options;
}
```

**Step 6: Update `build_chat_completions_request_body`**

Add after the existing parameter blocks:

```rust
if let Some(logprobs) = options.logprobs {
    json["logprobs"] = serde_json::json!(logprobs);
}

if let Some(top_logprobs) = options.top_logprobs {
    json["top_logprobs"] = serde_json::json!(top_logprobs);
}

if let Some(user) = &options.user {
    json["user"] = serde_json::json!(user);
}

if let Some(stream_options) = &options.stream_options {
    json["stream_options"] = serde_json::to_value(stream_options)
        .expect("OpenAiStreamOptions serialization cannot fail");
}
```

**Step 7: Update existing `request_body_omits_optional_fields_by_default` test**

Add assertions for new fields:

```rust
assert!(body.get("logprobs").is_none());
assert!(body.get("top_logprobs").is_none());
assert!(body.get("user").is_none());
assert!(body.get("stream_options").is_none());
```

**Step 8: Run all tests**

Run: `cargo test --lib --features api-clients -- openai::tests`
Expected: All 12 tests pass.

**Step 9: Commit**

```bash
git add src/clients/openai.rs
git commit -m "feat(openai): add logprobs, top_logprobs, user, and stream_options parameters"
```

---

### Task 5: Improve Doc Comments — add valid range annotations

**Files:**
- Modify: `src/clients/openai.rs` (doc comments on `OpenAiRequestOptions` fields and setters)

**Step 1: Update `OpenAiRequestOptions` field docs**

```rust
/// Sampling temperature (`0.0..=2.0`). Higher values produce more random output.
pub temperature: Option<f32>,
/// Nucleus sampling threshold (`0.0..=1.0`).
pub top_p: Option<f32>,
/// Maximum number of tokens to generate in the completion.
pub max_completion_tokens: Option<u32>,
/// Stop sequences (up to 4). Generation stops when any sequence is encountered.
pub stop: Option<OpenAiStop>,
/// Penalizes new tokens based on whether they appear in the text so far (`-2.0..=2.0`).
pub presence_penalty: Option<f32>,
/// Penalizes new tokens based on their frequency in the text so far (`-2.0..=2.0`).
pub frequency_penalty: Option<f32>,
```

**Step 2: Update setter method docs similarly**

Mirror the range info in each `set_*` method's doc comment.

**Step 3: Run `cargo doc --no-deps --features api-clients`**

Expected: No warnings.

**Step 4: Commit**

```bash
git add src/clients/openai.rs
git commit -m "docs(openai): add valid range annotations to parameter doc comments"
```

---

### Task 6: Update Example

**Files:**
- Modify: `examples/openai-standardized/src/main.rs`

**Step 1: Update example to demonstrate new parameters**

Add a brief section in `run_structured_output_test` that sets `user`:

```rust
client.set_user(Some("example-user".to_string()));
```

**Step 2: Verify example compiles**

Run: `cargo build --manifest-path examples/openai-standardized/Cargo.toml`
Expected: Compiles without errors.

**Step 3: Commit**

```bash
git add examples/openai-standardized/src/main.rs
git commit -m "chore(example): demonstrate user parameter in standardized example"
```
