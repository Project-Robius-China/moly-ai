use async_stream::stream;
use moly_kit::aitk::utils::sse::parse_sse;
use moly_kit::prelude::*;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// SSE event from the Octos gateway.
///
/// Octos uses a custom (non-OpenAI) SSE protocol. Streaming is triggered by
/// `POST /api/chat` with `"stream": true`. Event types observed:
/// `response`, `token`, `tool_start`, `tool_end`, `cost_update`,
/// `stream_end`, `done`.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
enum OctosEvent {
    /// Signals the start of a response iteration.
    #[serde(rename = "response")]
    Response { iteration: u32 },
    /// A streamed text token.
    #[serde(rename = "token")]
    Token { text: String },
    /// A tool invocation has started.
    #[serde(rename = "tool_start")]
    ToolStart { tool: String },
    /// A tool invocation has completed.
    #[serde(rename = "tool_end")]
    ToolEnd { tool: String, success: bool },
    /// Token usage and cost update.
    #[serde(rename = "cost_update")]
    CostUpdate {
        input_tokens: u64,
        output_tokens: u64,
        session_cost: f64,
    },
    /// The token stream has ended (final content follows in `done`).
    #[serde(rename = "stream_end")]
    StreamEnd,
    /// Final summary with the complete assembled content.
    #[serde(rename = "done")]
    Done {
        #[allow(dead_code)]
        content: String,
        #[allow(dead_code)]
        input_tokens: u64,
        #[allow(dead_code)]
        output_tokens: u64,
    },
}

/// Cost metadata stored in `MessageContent.data`.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct CostData {
    input_tokens: u64,
    output_tokens: u64,
    session_cost: f64,
}

#[derive(Clone, Debug)]
struct OctosClientInner {
    url: String,
    headers: HeaderMap,
    client: reqwest::Client,
}

/// A client for interacting with the Octos gateway.
///
/// Octos uses a custom SSE protocol (not OpenAI-compatible).
/// A single `POST /api/chat` with `"stream": true` returns an SSE stream.
/// This client translates Octos events into aitk's `MessageContent` stream.
#[derive(Debug)]
pub struct OctosClient(Arc<RwLock<OctosClientInner>>);

impl Clone for OctosClient {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl From<OctosClientInner> for OctosClient {
    fn from(inner: OctosClientInner) -> Self {
        Self(Arc::new(RwLock::new(inner)))
    }
}

impl OctosClient {
    /// Creates a new client with the given Octos gateway base URL.
    pub fn new(url: String) -> Self {
        OctosClientInner {
            url,
            headers: HeaderMap::new(),
            client: default_client(),
        }
        .into()
    }

    /// Sets the Bearer token for authentication.
    pub fn set_key(&mut self, key: &str) -> Result<(), &'static str> {
        let value = format!("Bearer {}", key)
            .parse()
            .map_err(|_| "Invalid header value")?;
        self.0
            .write()
            .expect("OctosClient lock poisoned")
            .headers
            .insert(reqwest::header::AUTHORIZATION, value);
        Ok(())
    }
}

impl BotClient for OctosClient {
    fn bots(&mut self) -> BoxPlatformSendFuture<'static, ClientResult<Vec<Bot>>> {
        let bot = Bot {
            id: BotId::new("Octos"),
            name: "Octos Agent".to_string(),
            avatar: EntityAvatar::Text("C".into()),
            capabilities: BotCapabilities::new()
                .with_capabilities([BotCapability::TextInput]),
        };

        Box::pin(async move { ClientResult::new_ok(vec![bot]) })
    }

    fn clone_box(&self) -> Box<dyn BotClient> {
        Box::new(self.clone())
    }

    fn send(
        &mut self,
        _bot_id: &BotId,
        messages: &[Message],
        _tools: &[Tool],
    ) -> BoxPlatformSendStream<'static, ClientResult<MessageContent>> {
        let inner = self.0.read().expect("OctosClient lock poisoned").clone();

        // Extract the last user message as the chat payload
        let user_message = messages
            .iter()
            .rev()
            .find(|m| m.from == EntityId::User)
            .map(|m| m.content.text.clone())
            .unwrap_or_default();

        let chat_url = format!("{}/api/chat", inner.url);
        let headers = inner.headers.clone();

        let stream = stream! {
            // POST /api/chat with stream: true to get SSE response
            let response = match inner
                .client
                .post(&chat_url)
                .headers(headers)
                .json(&serde_json::json!({
                    "message": user_message,
                    "stream": true,
                }))
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => response,
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    yield ClientError::new(
                        ClientErrorKind::Response,
                        format!("POST /api/chat failed with status {status}"),
                    )
                    .with_details(body)
                    .into();
                    return;
                }
                Err(error) => {
                    log::error!(
                        "Failed to POST /api/chat at {chat_url}: {error:?}"
                    );
                    yield ClientError::new_with_source(
                        ClientErrorKind::Network,
                        format!("Failed to connect to Octos at {chat_url}"),
                        Some(error),
                    )
                    .into();
                    return;
                }
            };

            // Parse SSE events and build MessageContent
            let events = parse_sse(response.bytes_stream());
            let mut content = MessageContent::default();
            let mut message_count: u32 = 0;
            let yield_frequency = 10;

            for await event in events {
                let event = match event {
                    Ok(chunk) => {
                        message_count += 1;
                        chunk
                    }
                    Err(error) => {
                        log::error!(
                            "SSE stream error from {chat_url}: {error:?}"
                        );
                        yield ClientError::new_with_source(
                            ClientErrorKind::Network,
                            format!(
                                "Connection lost while streaming from \
                                 {chat_url}"
                            ),
                            Some(error),
                        )
                        .into();
                        return;
                    }
                };

                let octos_event: OctosEvent =
                    match serde_json::from_str(&event) {
                        Ok(e) => e,
                        Err(error) => {
                            log::error!(
                                "Failed to parse Octos SSE event: \
                                 {error}\nEvent content: {event}"
                            );
                            yield ClientError::new_with_source(
                                ClientErrorKind::Format,
                                "Could not parse Octos SSE event as JSON"
                                    .to_string(),
                                Some(error),
                            )
                            .into();
                            return;
                        }
                    };

                match octos_event {
                    OctosEvent::Token { text } => {
                        content.text.push_str(&text);
                    }
                    OctosEvent::Response { iteration } => {
                        if !content.reasoning.is_empty() {
                            content.reasoning.push('\n');
                        }
                        content.reasoning.push_str(
                            &format!("[Response iteration {iteration}]"),
                        );
                    }
                    OctosEvent::ToolStart { ref tool } => {
                        content.tool_calls.push(ToolCall {
                            id: format!("octos-{tool}"),
                            name: tool.clone(),
                            arguments: serde_json::Map::new(),
                            permission_status:
                                ToolCallPermissionStatus::default(),
                        });
                    }
                    OctosEvent::ToolEnd { ref tool, success } => {
                        content.tool_results.push(ToolResult {
                            tool_call_id: format!("octos-{tool}"),
                            content: if success {
                                format!("Tool '{tool}' completed successfully")
                            } else {
                                format!("Tool '{tool}' failed")
                            },
                            is_error: !success,
                        });
                    }
                    OctosEvent::CostUpdate {
                        input_tokens,
                        output_tokens,
                        session_cost,
                    } => {
                        let cost = CostData {
                            input_tokens,
                            output_tokens,
                            session_cost,
                        };
                        content.data = Some(
                            serde_json::to_string(&cost)
                                .unwrap_or_default(),
                        );
                    }
                    OctosEvent::StreamEnd => {}
                    OctosEvent::Done { .. } => {
                        break;
                    }
                }

                // Yield periodically to reduce back-pressure
                if message_count.is_multiple_of(yield_frequency)
                    || message_count < 20
                {
                    yield ClientResult::new_ok(content.clone());
                }
            }

            // Final yield to ensure the last state is captured
            yield ClientResult::new_ok(content);
        };

        Box::pin(stream)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn default_client() -> reqwest::Client {
    use std::time::Duration;

    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_secs(360))
        .build()
        .expect("Failed to build reqwest client")
}

#[cfg(target_arch = "wasm32")]
fn default_client() -> reqwest::Client {
    reqwest::Client::new()
}
