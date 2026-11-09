use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use copilot_core::{
    AnswerFailure, AnswerKey, AnswerMode, AnswerRequest, MAX_ANSWER_BYTES, Source, Span,
};
use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::{self, ChatGpt},
    platform::protect_settings,
};

const OPENAI: &str = "https://api.openai.com/v1";
const CLAUDE: &str = "https://api.anthropic.com/v1";
const OLLAMA: &str = "http://127.0.0.1:11434";
const MAX_EVENT_BYTES: usize = 256_000;
const MAX_CATALOG_BYTES: usize = 2_000_000;
pub const ANSWER_TIMEOUT_MS: u64 = 120_000;
const INSTRUCTIONS: &str = "Answer the current question directly and concisely. Use relevant prior turns for follow-ups. State assumptions when needed. Do not invent the user's experience. Use short paragraphs and numbered steps when helpful.";

pub fn one_off(question: String) -> AnswerRequest {
    AnswerRequest {
        key: AnswerKey {
            session_id: 0,
            question_id: 0,
            revision: 1,
            mode: AnswerMode::Checked,
        },
        question,
        source: Source::TYPED,
        span: Span::at(0),
        follow_up: None,
    }
}

#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Chatgpt,
    Claude,
    Ollama,
}

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct Selection {
    pub kind: ProviderKind,
    pub model: String,
}

#[derive(Default, Clone, Deserialize, Serialize)]
#[serde(default)]
struct Settings {
    host: String,
    chatgpt: ChatGpt,
    claude_key: Option<String>,
    selected: Option<Selection>,
}

#[derive(Serialize)]
pub struct Connections {
    chatgpt_account: Option<String>,
    chatgpt_registered: bool,
    claude_key_saved: bool,
    error: Option<String>,
}

#[derive(Serialize)]
pub struct Model {
    pub id: String,
    name: String,
}

pub struct ProviderHub {
    client: Client,
    path: PathBuf,
    settings: Mutex<Settings>,
    load_error: Option<String>,
    sign_in: tokio::sync::Mutex<()>,
    renew: tokio::sync::Mutex<()>,
    attempt: AtomicU64,
    ollama_origin: String,
}

impl ProviderHub {
    pub fn new(directory: PathBuf) -> Result<Self, String> {
        let path = directory.join("providers.dpapi");
        let loaded = match fs::read(&path) {
            Ok(bytes) if bytes.len() <= MAX_CATALOG_BYTES => protect_settings(&bytes, false)
                .and_then(|plain| {
                    serde_json::from_slice(&plain)
                        .map_err(|_| "Provider settings could not be read.".into())
                }),
            Ok(_) => Err("Provider settings are too large.".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(_) => Err("Provider settings could not be opened.".into()),
        };
        let (settings, load_error) = match loaded {
            Ok(settings) => (settings, None),
            Err(error) => (Settings::default(), Some(error)),
        };
        let client = Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_millis(ANSWER_TIMEOUT_MS - 5_000))
            .build()
            .map_err(|_| "Could not initialize provider networking.")?;
        Ok(Self {
            client,
            path,
            settings: Mutex::new(settings),
            load_error,
            sign_in: tokio::sync::Mutex::new(()),
            renew: tokio::sync::Mutex::new(()),
            attempt: AtomicU64::new(0),
            ollama_origin: OLLAMA.into(),
        })
    }

    fn settings(&self) -> Settings {
        self.settings
            .lock()
            .expect("provider settings lock")
            .clone()
    }

    fn change<T>(
        &self,
        modify: impl FnOnce(&mut Settings) -> Result<T, String>,
    ) -> Result<T, String> {
        if let Some(error) = &self.load_error {
            return Err(error.clone());
        }
        let mut stored = self.settings.lock().expect("provider settings lock");
        let mut next = stored.clone();
        let value = modify(&mut next)?;
        let plain = serde_json::to_vec(&next).map_err(|_| "Could not encode provider settings.")?;
        let sealed = protect_settings(&plain, true)?;
        let directory = self
            .path
            .parent()
            .ok_or("Provider settings directory is missing.")?;
        fs::create_dir_all(directory)
            .map_err(|_| "Could not create provider settings directory.")?;
        let mut file = tempfile::NamedTempFile::new_in(directory)
            .map_err(|_| "Could not create protected provider settings.")?;
        file.write_all(&sealed)
            .and_then(|()| file.as_file().sync_all())
            .map_err(|_| "Could not write protected provider settings.")?;
        file.persist(&self.path)
            .map_err(|_| "Could not replace protected provider settings.")?;
        *stored = next;
        Ok(value)
    }

    pub fn selection(&self) -> Option<Selection> {
        self.settings
            .lock()
            .expect("provider settings lock")
            .selected
            .clone()
    }

    pub fn connections(&self) -> Connections {
        let settings = self.settings.lock().expect("provider settings lock");
        Connections {
            chatgpt_account: settings.chatgpt.account().map(str::to_owned),
            chatgpt_registered: settings.chatgpt.registered(),
            claude_key_saved: settings.claude_key.is_some(),
            error: self.load_error.clone(),
        }
    }

    pub fn select(&self, mut route: Selection, consent: bool) -> Result<(), String> {
        route.model = route.model.trim().to_owned();
        if !model_id(&route.model) {
            return Err("Choose a model or enter a valid model ID.".into());
        }
        if route.kind != ProviderKind::Ollama && !consent {
            return Err(
                "Confirm that questions and linked context may be sent to this provider.".into(),
            );
        }
        if route.kind == ProviderKind::Ollama && cloud_tag(&route.model) {
            return Err("Choose an installed local Ollama model.".into());
        }
        self.change(|settings| {
            match route.kind {
                ProviderKind::Chatgpt if settings.chatgpt.account().is_none() => {
                    return Err("Continue with ChatGPT before selecting this provider.".into());
                }
                ProviderKind::Claude if settings.claude_key.is_none() => {
                    return Err("Save your Claude Console API key first.".into());
                }
                _ => {}
            }
            settings.selected = Some(route);
            Ok(())
        })
    }

    pub fn disable(&self) -> Result<(), String> {
        self.change(|settings| {
            settings.selected = None;
            Ok(())
        })
    }

    pub fn save_claude(&self, key: String) -> Result<(), String> {
        let key = valid_claude_key(&key)?;
        self.change(|settings| {
            settings.claude_key = Some(key.to_owned());
            deselect(settings, ProviderKind::Claude);
            Ok(())
        })
    }

    pub fn clear_claude(&self) -> Result<(), String> {
        self.change(|settings| {
            settings.claude_key = None;
            deselect(settings, ProviderKind::Claude);
            Ok(())
        })
    }

    pub async fn sign_in_chatgpt(&self, different_account: bool) -> Result<(), String> {
        let _login = self
            .sign_in
            .try_lock()
            .map_err(|_| "A ChatGPT sign-in is already in progress.")?;
        let attempt = self.attempt.load(Ordering::Acquire);
        let mut old = self.settings();
        if old.host.is_empty() {
            old.host = self.change(|settings| {
                if settings.host.is_empty() {
                    settings.host = format!("urn:uuid:{}", Uuid::new_v4());
                }
                Ok(settings.host.clone())
            })?;
        }
        let cancelled = async {
            while self.attempt.load(Ordering::Acquire) == attempt {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        };
        let next = tokio::select! {
            biased;
            () = cancelled => return Err("ChatGPT sign-in cancelled.".into()),
            result = auth::sign_in(&self.client, &old.host, &old.chatgpt, different_account) => result?,
        };
        let _refresh = self.renew.lock().await;
        if self.attempt.load(Ordering::Acquire) != attempt {
            return Err("ChatGPT sign-in cancelled.".into());
        }
        self.change(|settings| {
            settings.chatgpt = next;
            deselect(settings, ProviderKind::Chatgpt);
            Ok(())
        })
    }

    pub fn cancel_sign_in(&self) {
        self.attempt.fetch_add(1, Ordering::AcqRel);
    }

    pub async fn clear_chatgpt(&self) -> Result<ChatGpt, String> {
        self.cancel_sign_in();
        let _refresh = self.renew.lock().await;
        self.change(|settings| {
            let old = settings.chatgpt.clone();
            settings.chatgpt.clear();
            deselect(settings, ProviderKind::Chatgpt);
            Ok(old)
        })
    }

    pub async fn revoke_chatgpt(&self, old: ChatGpt) -> Result<(), String> {
        tokio::time::timeout(Duration::from_secs(10), old.revoke(&self.client))
            .await
            .map_err(
                |_| "Signed out locally. Check connected apps in ChatGPT to revoke account access.",
            )?
    }

    async fn bearer(&self) -> Result<String, String> {
        let _refresh = self.renew.lock().await;
        let old = self.settings().chatgpt;
        if let Some(access) = old.access() {
            return Ok(access.to_owned());
        }
        let next = old.renew(&self.client).await?;
        let access = next
            .access()
            .ok_or("Reconnect your ChatGPT account in Providers.")?
            .to_owned();
        self.change(|settings| {
            settings.chatgpt = next;
            Ok(())
        })?;
        Ok(access)
    }

    fn claude_key(&self) -> Result<String, String> {
        self.settings()
            .claude_key
            .ok_or("Save a Claude Console API key first.".into())
    }

    pub async fn models(&self, kind: ProviderKind) -> Result<Vec<Model>, String> {
        let request = match kind {
            ProviderKind::Chatgpt => self
                .client
                .get(format!("{OPENAI}/models"))
                .bearer_auth(self.bearer().await?),
            ProviderKind::Claude => self.claude_catalog(&self.claude_key()?),
            ProviderKind::Ollama => self.client.get(format!("{}/api/tags", self.ollama_origin)),
        };
        Self::read_catalog(request, kind).await
    }

    pub async fn validate_claude(&self, key: &str) -> Result<Vec<Model>, String> {
        let request = self.claude_catalog(valid_claude_key(key)?);
        Self::read_catalog(request, ProviderKind::Claude).await
    }

    fn claude_catalog(&self, key: &str) -> RequestBuilder {
        self.client
            .get(format!("{CLAUDE}/models"))
            .header("anthropic-version", "2023-06-01")
            .header("x-api-key", key)
    }

    async fn read_catalog(
        request: RequestBuilder,
        kind: ProviderKind,
    ) -> Result<Vec<Model>, String> {
        let response = request
            .timeout(Duration::from_secs(12))
            .send()
            .await
            .map_err(|_| {
                if kind == ProviderKind::Ollama {
                    "Start Ollama on this PC and try again."
                } else {
                    "Could not reach the answer provider."
                }
            })?;
        if !response.status().is_success() {
            let failure = status_failure(response.status());
            return Err(match (kind, failure) {
                (ProviderKind::Claude, AnswerFailure::AuthenticationRequired) => {
                    "Claude rejected this API key. Check the key in Claude Console."
                }
                (ProviderKind::Chatgpt, AnswerFailure::AuthenticationRequired) => {
                    "ChatGPT did not grant access to models. Reconnect and allow plan usage."
                }
                (_, failure) => failure.message(),
            }
            .into());
        }
        let response: Value = json_body(response).await?;
        Ok(model_catalog(kind, &response))
    }

    pub async fn answer(
        &self,
        route: &Selection,
        question: &AnswerRequest,
        emit: impl FnMut(&str) -> bool,
    ) -> Result<String, AnswerFailure> {
        self.answer_with(route, question, INSTRUCTIONS, emit).await
    }

    /// Embeds texts with a local Ollama model. Only on-device Ollama is used for this.
    pub async fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, String> {
        if !model_id(model) || cloud_tag(model) {
            return Err("Choose a local Ollama embedding model.".into());
        }
        let response = self
            .client
            .post(format!("{}/api/embed", self.ollama_origin))
            .json(&json!({ "model": model, "input": inputs }))
            .timeout(Duration::from_secs(60))
            .send()
            .await
            .map_err(|_| "Start Ollama on this PC and try again.")?;
        #[derive(Deserialize)]
        struct Embeddings {
            embeddings: Vec<Vec<f32>>,
        }
        let body: Embeddings = json_body(response).await?;
        if body.embeddings.len() != inputs.len() {
            return Err("Ollama returned the wrong number of embeddings.".into());
        }
        Ok(body.embeddings)
    }

    /// Same streaming request as [`Self::answer`], with task-specific instructions.
    pub async fn answer_with(
        &self,
        route: &Selection,
        question: &AnswerRequest,
        instructions: &str,
        emit: impl FnMut(&str) -> bool,
    ) -> Result<String, AnswerFailure> {
        if self.selection().as_ref() != Some(route) {
            return Err(AnswerFailure::AuthenticationRequired);
        }
        let context = messages(question);
        let request = match route.kind {
            ProviderKind::Chatgpt => {
                let access = self
                    .bearer()
                    .await
                    .map_err(|_| AnswerFailure::AuthenticationRequired)?;
                self.client
                    .post(format!("{OPENAI}/responses"))
                    .bearer_auth(access)
                    // Low effort keeps the first words of a live answer within a few seconds.
                    .json(&json!({
                        "model": route.model, "input": context, "instructions": instructions,
                        "store": false, "stream": true, "reasoning": { "effort": "low" },
                    }))
            }
            ProviderKind::Claude => {
                let key = self
                    .claude_key()
                    .map_err(|_| AnswerFailure::AuthenticationRequired)?;
                self.client
                    .post(format!("{CLAUDE}/messages"))
                    .header("anthropic-version", "2023-06-01")
                    .header("x-api-key", key)
                    .json(&json!({
                        "model": route.model, "messages": context, "system": instructions,
                        "max_tokens": 1_600, "stream": true,
                    }))
            }
            ProviderKind::Ollama => {
                let models = self
                    .models(ProviderKind::Ollama)
                    .await
                    .map_err(|_| AnswerFailure::Unavailable)?;
                if !models.iter().any(|model| model.id == route.model) {
                    return Err(AnswerFailure::Unavailable);
                }
                let mut context = context;
                context.insert(0, json!({"role": "system", "content": instructions}));
                self.client
                    .post(format!("{}/api/chat", self.ollama_origin))
                    .json(&json!({
                        "model": route.model, "messages": context, "stream": true,
                        "think": false, "options": { "num_predict": 1_600 },
                    }))
            }
        };
        // An account change can invalidate the route while a token or model catalog is loading.
        if self.selection().as_ref() != Some(route) {
            return Err(AnswerFailure::AuthenticationRequired);
        }
        let response = request.send().await.map_err(network_failure)?;
        if !response.status().is_success() {
            return Err(status_failure(response.status()));
        }
        consume(response, route.kind, emit).await
    }
}

fn valid_claude_key(key: &str) -> Result<&str, String> {
    let key = key.trim();
    if !(16..=512).contains(&key.len()) || !key.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err("Enter a valid Claude Console API key.".into());
    }
    Ok(key)
}

fn deselect(settings: &mut Settings, kind: ProviderKind) {
    if settings
        .selected
        .as_ref()
        .is_some_and(|route| route.kind == kind)
    {
        settings.selected = None;
    }
}

fn model_id(id: &str) -> bool {
    (1..=160).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./:".contains(&b))
}

fn cloud_tag(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    id.ends_with("-cloud") || id.ends_with(":cloud") || id.starts_with("ollama.com/")
}

fn model_catalog(kind: ProviderKind, response: &Value) -> Vec<Model> {
    let (list, id_field, name_field) = match kind {
        ProviderKind::Chatgpt => ("models", "slug", "display_name"),
        ProviderKind::Claude => ("data", "id", "display_name"),
        ProviderKind::Ollama => ("models", "name", "name"),
    };
    response[list]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let id = entry[id_field].as_str().filter(|id| model_id(id))?;
            if (kind == ProviderKind::Chatgpt && entry["visibility"] != "list")
                || (kind == ProviderKind::Ollama
                    && (cloud_tag(id)
                        || entry["size"].as_u64().unwrap_or(0) == 0
                        || entry["remote_host"]
                            .as_str()
                            .is_some_and(|host| !host.is_empty())
                        || entry["remote_model"]
                            .as_str()
                            .is_some_and(|name| !name.is_empty())))
            {
                return None;
            }
            let name = entry[name_field]
                .as_str()
                .filter(|name| name.len() <= 240)
                .unwrap_or(id);
            Some(Model {
                id: id.to_owned(),
                name: name.to_owned(),
            })
        })
        .take(160)
        .collect()
}

fn messages(question: &AnswerRequest) -> Vec<Value> {
    let mut messages = Vec::with_capacity(3);
    if let Some(prior) = &question.follow_up {
        let previous = prefix(&prior.question, 8_000);
        if let Some(answer) = &prior.checked_answer {
            messages.push(json!({"role": "user", "content": previous}));
            messages.push(json!({"role": "assistant", "content": prefix(answer, 12_000)}));
        } else {
            messages.push(json!({"role": "user", "content": format!("Previous question for context: {previous}\n\nCurrent question: {}", question.question)}));
            return messages;
        }
    }
    messages.push(json!({"role": "user", "content": question.question}));
    messages
}

fn prefix(text: &str, limit: usize) -> &str {
    let mut end = text.len().min(limit);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

fn status_failure(status: StatusCode) -> AnswerFailure {
    match status.as_u16() {
        401 | 403 => AnswerFailure::AuthenticationRequired,
        402 | 429 => AnswerFailure::UsageLimit,
        408 | 504 => AnswerFailure::TimedOut,
        _ => AnswerFailure::Unavailable,
    }
}

fn network_failure(error: reqwest::Error) -> AnswerFailure {
    if error.is_timeout() {
        AnswerFailure::TimedOut
    } else {
        AnswerFailure::Unavailable
    }
}

pub async fn json_body<T: DeserializeOwned>(mut response: Response) -> Result<T, String> {
    if !response.status().is_success() {
        return Err(status_failure(response.status()).message().into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| "Could not read the provider response.")?
    {
        if chunk.len() > MAX_CATALOG_BYTES.saturating_sub(bytes.len()) {
            return Err("Provider response is too large.".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| "The provider returned invalid data.".into())
}

enum StreamEvent<'a> {
    Text(&'a str),
    Done,
    Skip,
}

fn event(kind: ProviderKind, data: &Value) -> Result<StreamEvent<'_>, AnswerFailure> {
    let kind_name = data["type"].as_str().unwrap_or("");
    let text = match kind {
        ProviderKind::Chatgpt => match kind_name {
            "response.completed" => return Ok(StreamEvent::Done),
            "response.failed" | "response.incomplete" | "error" => {
                // A plan limit arrives as {"type":"error","error":{"code":…}} before response.failed.
                let code = data["response"]["error"]["code"]
                    .as_str()
                    .or(data["error"]["code"].as_str())
                    .or(data["code"].as_str());
                return Err(
                    if code.is_some_and(|code| {
                        code.starts_with("subscription_sharing_usage_")
                            || ["rate_limit_exceeded", "insufficient_quota"].contains(&code)
                    }) {
                        AnswerFailure::UsageLimit
                    } else {
                        AnswerFailure::InvalidOutput
                    },
                );
            }
            "response.output_text.delta" => data["delta"].as_str(),
            _ => None,
        },
        ProviderKind::Claude => match kind_name {
            "message_stop" => return Ok(StreamEvent::Done),
            "message_delta" if data["delta"]["stop_reason"] == "max_tokens" => {
                return Err(AnswerFailure::InvalidOutput);
            }
            "error" => {
                return Err(if data["error"]["type"] == "rate_limit_error" {
                    AnswerFailure::UsageLimit
                } else {
                    AnswerFailure::Unavailable
                });
            }
            "content_block_delta" if data["delta"]["type"] == "text_delta" => {
                data["delta"]["text"].as_str()
            }
            _ => None,
        },
        ProviderKind::Ollama => {
            if data["error"].is_string() {
                return Err(AnswerFailure::Unavailable);
            }
            if data["done"] == true {
                return Ok(StreamEvent::Done);
            }
            data["message"]["content"].as_str()
        }
    };
    Ok(text.map_or(StreamEvent::Skip, StreamEvent::Text))
}

async fn consume(
    mut response: Response,
    kind: ProviderKind,
    mut emit: impl FnMut(&str) -> bool,
) -> Result<String, AnswerFailure> {
    let mut pending = Vec::new();
    let mut frame = String::new();
    let mut text = String::new();
    let mut sent = 0;
    let mut last_send = Instant::now();
    while let Some(chunk) = response.chunk().await.map_err(network_failure)? {
        if chunk.len() > MAX_EVENT_BYTES.saturating_sub(pending.len()) {
            return Err(AnswerFailure::InvalidOutput);
        }
        pending.extend_from_slice(&chunk);
        let mut complete = 0;
        for bytes in pending.split_inclusive(|byte| *byte == b'\n') {
            if bytes.last() != Some(&b'\n') {
                break;
            }
            complete += bytes.len();
            let line = std::str::from_utf8(bytes)
                .map_err(|_| AnswerFailure::InvalidOutput)?
                .trim_end_matches(['\r', '\n']);
            let data = if kind == ProviderKind::Ollama {
                line
            } else {
                if let Some(part) = line.strip_prefix("data:") {
                    let part = part.strip_prefix(' ').unwrap_or(part);
                    if frame.len() + part.len() + 1 > MAX_EVENT_BYTES {
                        return Err(AnswerFailure::InvalidOutput);
                    }
                    if !frame.is_empty() {
                        frame.push('\n');
                    }
                    frame.push_str(part);
                }
                if !line.is_empty() {
                    continue;
                }
                frame.as_str()
            };
            if data.is_empty() || data == "[DONE]" {
                frame.clear();
                continue;
            }
            let value: Value =
                serde_json::from_str(data).map_err(|_| AnswerFailure::InvalidOutput)?;
            frame.clear();
            match event(kind, &value)? {
                StreamEvent::Skip => {}
                StreamEvent::Done => {
                    return if text.trim().is_empty() {
                        Err(AnswerFailure::InvalidOutput)
                    } else {
                        Ok(text)
                    };
                }
                StreamEvent::Text(delta) => {
                    if delta.len() > MAX_ANSWER_BYTES.saturating_sub(text.len()) {
                        return Err(AnswerFailure::InvalidOutput);
                    }
                    text.push_str(delta);
                    if sent == 0
                        || text.len() - sent >= 1_024
                        || last_send.elapsed() >= Duration::from_millis(55)
                    {
                        if !emit(&text[sent..]) {
                            return Err(AnswerFailure::Unavailable);
                        }
                        sent = text.len();
                        last_send = Instant::now();
                    }
                }
            }
        }
        pending.drain(..complete);
    }
    Err(AnswerFailure::InvalidOutput)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn stream_fixture(
        kind: ProviderKind,
        body: &'static str,
    ) -> Result<String, AnswerFailure> {
        let server = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
        let address = server.local_addr().unwrap();
        let worker = tokio::spawn(async move {
            let (mut stream, _) = server.accept().await.unwrap();
            let mut request = [0; 4096];
            assert!(stream.read(&mut request).await.unwrap() > 0);
            let headers = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
            for chunk in body.as_bytes().chunks(13) {
                stream.write_all(chunk).await.unwrap();
                tokio::task::yield_now().await;
            }
        });
        let response = Client::new()
            .get(format!("http://{address}"))
            .send()
            .await
            .unwrap();
        let mut preview = String::new();
        let result = consume(response, kind, |delta| {
            preview.push_str(delta);
            true
        })
        .await;
        worker.await.unwrap();
        if result.is_ok() {
            assert!(!preview.is_empty());
        }
        result
    }

    #[tokio::test]
    async fn all_three_streams_require_their_own_completion_event() {
        let openai = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Café\"}\n\n";
        assert_eq!(
            stream_fixture(ProviderKind::Chatgpt, openai).await,
            Err(AnswerFailure::InvalidOutput)
        );
        let openai = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Café\"}\n\ndata: {\"type\":\"response.completed\"}\n\n";
        assert_eq!(
            stream_fixture(ProviderKind::Chatgpt, openai).await.unwrap(),
            "Café"
        );
        // Shape observed from a ChatGPT plan whose connected-app allowance is used up.
        let limited = "data: {\"type\":\"error\",\"error\":{\"type\":\"invalid_request_error\",\"code\":\"subscription_sharing_usage_limit_exceeded\"}}\n\n";
        assert_eq!(
            stream_fixture(ProviderKind::Chatgpt, limited).await,
            Err(AnswerFailure::UsageLimit)
        );
        let claude = "event: content_block_delta\r\ndata: {\"type\":\"content_block_delta\",\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\r\n\r\ndata: {\"type\":\"message_stop\"}\r\n\r\n";
        assert_eq!(
            stream_fixture(ProviderKind::Claude, claude).await.unwrap(),
            "Hello"
        );
        let ollama = "{\"message\":{\"content\":\"Local\"},\"done\":false}\n{\"done\":true}\n";
        assert_eq!(
            stream_fixture(ProviderKind::Ollama, ollama).await.unwrap(),
            "Local"
        );
    }

    #[test]
    fn ollama_catalog_filters_cloud_aliases_and_zero_size() {
        let data = json!({"models":[
            {"name":"qwen3:4b","size":42},
            {"name":"alias","size":42,"remote_host":"https://ollama.com"},
            {"name":"gpt-oss:cloud","size":42},
            {"name":"removed","size":0},
        ]});
        let models = model_catalog(ProviderKind::Ollama, &data);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "qwen3:4b");
    }

    #[cfg(windows)]
    #[test]
    fn claude_key_is_persisted_as_protected_bytes_and_cloud_requires_consent() {
        let dir = tempfile::tempdir().unwrap();
        let hub = ProviderHub::new(dir.path().into()).unwrap();
        let route = Selection {
            kind: ProviderKind::Claude,
            model: "claude-test".into(),
        };
        hub.save_claude("synthetic-test-credential".into()).unwrap();
        assert!(hub.select(route.clone(), false).is_err());
        hub.select(route, true).unwrap();
        let bytes = fs::read(dir.path().join("providers.dpapi")).unwrap();
        assert!(
            !bytes
                .windows(b"synthetic-test-credential".len())
                .any(|part| part == b"synthetic-test-credential")
        );
        let reopened = ProviderHub::new(dir.path().into()).unwrap();
        assert!(reopened.connections().claude_key_saved);
        assert!(reopened.selection().is_some());
        reopened.clear_claude().unwrap();
        assert!(reopened.selection().is_none());
    }
}
