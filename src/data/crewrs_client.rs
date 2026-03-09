use async_stream::stream;
use moly_kit::aitk::utils::sse::parse_sse;
use moly_kit::prelude::*;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// SSE event from the crew-rs gateway.
///
/// crew-rs uses a custom (non-OpenAI) SSE protocol with typed events:
/// `thinking`, `token`, `tool_start`, `tool_end`, `cost_update`, `stream_end`.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type")]
enum CrewRsEvent {
    #[serde(rename = "thinking")]
    Thinking { iteration: u32 },
    #[serde(rename = "token")]
    Token { text: String },
    #[serde(rename = "tool_start")]
    ToolStart { tool: String },
    #[serde(rename = "tool_end")]
    ToolEnd {
        tool: String,
        success: bool,
    },
    #[serde(rename = "cost_update")]
    CostUpdate {
        input_tokens: u64,
        output_tokens: u64,
        session_cost: f64,
    },
    #[serde(rename = "stream_end")]
    StreamEnd,
}

/// Cost metadata stored in `MessageContent.data`.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
struct CostData {
    input_tokens: u64,
    output_tokens: u64,
    session_cost: f64,
}

#[derive(Clone, Debug)]
struct CrewRsClientInner {
    url: String,
    headers: HeaderMap,
    client: reqwest::Client,
}

/// A client for interacting with the crew-rs gateway.
///
/// crew-rs uses a custom SSE protocol (not OpenAI-compatible).
/// This client translates crew-rs events into aitk's `MessageContent` stream.
#[derive(Debug)]
pub struct CrewRsClient(Arc<RwLock<CrewRsClientInner>>);

impl Clone for CrewRsClient {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl From<CrewRsClientInner> for CrewRsClient {
    fn from(inner: CrewRsClientInner) -> Self {
        Self(Arc::new(RwLock::new(inner)))
    }
}

impl CrewRsClient {
    /// Creates a new client with the given crew-rs gateway base URL.
    pub fn new(url: String) -> Self {
        CrewRsClientInner {
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
            .expect("CrewRsClient lock poisoned")
            .headers
            .insert(reqwest::header::AUTHORIZATION, value);
        Ok(())
    }
}

impl BotClient for CrewRsClient {
    fn bots(&mut self) -> BoxPlatformSendFuture<'static, ClientResult<Vec<Bot>>> {
        let bot = Bot {
            id: BotId::new("crew-rs"),
            name: "CrewRs Agent".to_string(),
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
        let inner = self.0.read().expect("CrewRsClient lock poisoned").clone();

        // Extract the last user message as the chat payload
        let user_message = messages
            .iter()
            .rev()
            .find(|m| m.from == EntityId::User)
            .map(|m| m.content.text.clone())
            .unwrap_or_default();

        let chat_url = format!("{}/api/chat", inner.url);
        let stream_url = format!("{}/api/chat/stream", inner.url);
        let headers = inner.headers.clone();

        let chat_body = serde_json::json!({
            "message": user_message,
        });

        let stream = stream! {
            // Step 1: POST /api/chat to initiate the conversation
            let post_result = inner
                .client
                .post(&chat_url)
                .headers(headers.clone())
                .json(&chat_body)
                .send()
                .await;

            match post_result {
                Ok(response) if !response.status().is_success() => {
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
                    log::error!("Failed to POST /api/chat at {chat_url}: {error:?}");
                    yield ClientError::new_with_source(
                        ClientErrorKind::Network,
                        format!("Failed to connect to crew-rs at {chat_url}"),
                        Some(error),
                    )
                    .into();
                    return;
                }
                Ok(_) => {}
            }

            // Step 2: GET /api/chat/stream to receive SSE events
            let stream_request = inner
                .client
                .get(&stream_url)
                .headers(headers);

            let response = match stream_request.send().await {
                Ok(response) if response.status().is_success() => response,
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    yield ClientError::new(
                        ClientErrorKind::Response,
                        format!("GET /api/chat/stream failed with status {status}"),
                    )
                    .with_details(body)
                    .into();
                    return;
                }
                Err(error) => {
                    log::error!(
                        "Failed to connect to SSE stream at {stream_url}: {error:?}"
                    );
                    yield ClientError::new_with_source(
                        ClientErrorKind::Network,
                        format!(
                            "Failed to connect to crew-rs SSE stream at {stream_url}"
                        ),
                        Some(error),
                    )
                    .into();
                    return;
                }
            };

            // Step 3: Parse SSE events and build MessageContent
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
                            "SSE stream error from {stream_url}: {error:?}"
                        );
                        yield ClientError::new_with_source(
                            ClientErrorKind::Network,
                            format!(
                                "Connection lost while streaming from {stream_url}"
                            ),
                            Some(error),
                        )
                        .into();
                        return;
                    }
                };

                let crew_event: CrewRsEvent = match serde_json::from_str(&event) {
                    Ok(e) => e,
                    Err(error) => {
                        log::error!(
                            "Failed to parse crew-rs SSE event: {error}\n\
                             Event content: {event}"
                        );
                        yield ClientError::new_with_source(
                            ClientErrorKind::Format,
                            format!(
                                "Could not parse crew-rs SSE event as JSON"
                            ),
                            Some(error),
                        )
                        .into();
                        return;
                    }
                };

                match crew_event {
                    CrewRsEvent::Token { text } => {
                        content.text.push_str(&text);
                    }
                    CrewRsEvent::Thinking { iteration } => {
                        if !content.reasoning.is_empty() {
                            content.reasoning.push('\n');
                        }
                        content
                            .reasoning
                            .push_str(&format!("[Thinking iteration {iteration}]"));
                    }
                    CrewRsEvent::ToolStart { ref tool } => {
                        content.tool_calls.push(ToolCall {
                            id: format!("crewrs-{tool}"),
                            name: tool.clone(),
                            arguments: serde_json::Map::new(),
                            permission_status: ToolCallPermissionStatus::default(),
                        });
                    }
                    CrewRsEvent::ToolEnd { ref tool, success } => {
                        content.tool_results.push(ToolResult {
                            tool_call_id: format!("crewrs-{tool}"),
                            content: if success {
                                format!("Tool '{tool}' completed successfully")
                            } else {
                                format!("Tool '{tool}' failed")
                            },
                            is_error: !success,
                        });
                    }
                    CrewRsEvent::CostUpdate {
                        input_tokens,
                        output_tokens,
                        session_cost,
                    } => {
                        let cost = CostData {
                            input_tokens,
                            output_tokens,
                            session_cost,
                        };
                        content.data =
                            Some(serde_json::to_string(&cost).unwrap_or_default());
                    }
                    CrewRsEvent::StreamEnd => {
                        break;
                    }
                }

                // Yield periodically to reduce back-pressure
                if message_count % yield_frequency == 0 || message_count < 20 {
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
