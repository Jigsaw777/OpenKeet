//! Opt-in, text-only Ollama compatibility for a local desktop client.

use std::{
    convert::Infallible,
    ffi::{OsStr, OsString},
    io,
    net::{Ipv4Addr, TcpListener},
    sync::Arc,
    time::Duration,
};

use copilot_core::MAX_QUESTION_BYTES;
use futures_util::stream;
use http_body_util::{
    BodyExt, Full, LengthLimitError, Limited, StreamBody, combinators::UnsyncBoxBody,
};
use hyper::{
    Method, Request, Response, StatusCode,
    body::{Bytes, Frame, Incoming},
    header::{HOST, ORIGIN},
    http::uri::Authority,
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{Semaphore, mpsc};

use crate::providers::{ProviderHub, one_off};

const MAX_REQUEST_BYTES: usize = 150_000;
type Body = UnsyncBoxBody<Bytes, Infallible>;

pub fn start(mut args: impl Iterator<Item = OsString>, hub: Arc<ProviderHub>) -> io::Result<()> {
    if args.next().as_deref() != Some(OsStr::new("--text-bridge")) {
        return Ok(());
    }
    let port = args.next().map_or(Some(11435), |value| {
        value
            .to_str()?
            .parse::<u16>()
            .ok()
            .filter(|port| *port != 0)
    });
    let port = port
        .filter(|_| args.next().is_none())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Use --text-bridge [port]."))?;
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, port))?;
    listener.set_nonblocking(true)?;
    tauri::async_runtime::spawn(async move {
        let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
            return;
        };
        let active = Arc::new(Semaphore::new(1));
        let connections = Arc::new(Semaphore::new(16));
        while let Ok((socket, _)) = listener.accept().await {
            let Ok(slot) = Arc::clone(&connections).try_acquire_owned() else {
                continue;
            };
            let hub = Arc::clone(&hub);
            let active = Arc::clone(&active);
            tokio::spawn(async move {
                let _slot = slot;
                let service = service_fn(move |request| {
                    let hub = Arc::clone(&hub);
                    let active = Arc::clone(&active);
                    async { Ok::<_, Infallible>(handle(request, hub, active).await) }
                });
                let _ = http1::Builder::new()
                    .timer(TokioTimer::new())
                    .header_read_timeout(Duration::from_secs(10))
                    .keep_alive(false)
                    .serve_connection(TokioIo::new(socket), service)
                    .await;
            });
        }
    });
    Ok(())
}

fn response(status: StatusCode, content_type: &'static str, body: Body) -> Response<Body> {
    Response::builder()
        .status(status)
        .header("content-type", content_type)
        .header("cache-control", "no-store")
        .header("x-content-type-options", "nosniff")
        .body(body)
        .expect("static HTTP response headers")
}

fn json_response(status: StatusCode, value: Value) -> Response<Body> {
    response(
        status,
        "application/json",
        Full::new(Bytes::from(value.to_string())).boxed_unsync(),
    )
}

fn error(status: StatusCode, message: &str) -> Response<Body> {
    json_response(status, json!({ "error": message }))
}

#[derive(Deserialize)]
struct Generate {
    prompt: String,
    model: Option<String>,
    #[serde(default)]
    stream: bool,
    images: Option<Vec<Value>>,
    audio: Option<Value>,
}

async fn handle(
    request: Request<Incoming>,
    hub: Arc<ProviderHub>,
    active: Arc<Semaphore>,
) -> Response<Body> {
    let local_host = request
        .headers()
        .get(HOST)
        .and_then(|host| host.to_str().ok())
        .and_then(|host| host.parse::<Authority>().ok())
        .is_some_and(|host| {
            host.host().eq_ignore_ascii_case("localhost") || host.host() == "127.0.0.1"
        });
    if !local_host || request.headers().contains_key(ORIGIN) {
        return error(
            StatusCode::FORBIDDEN,
            "Only local desktop clients can use the text bridge.",
        );
    }
    let route = hub.selection();
    match (request.method(), request.uri().path()) {
        (&Method::GET, "/api/version") => {
            return json_response(StatusCode::OK, json!({ "version": "openkeet-bridge-2" }));
        }
        (&Method::GET, "/api/tags") => {
            let models: Vec<_> = route
                .iter()
                .map(|route| {
                    json!({
                        "name": route.model, "model": route.model, "size": 0,
                        "details": { "family": "openkeet-text-bridge" }
                    })
                })
                .collect();
            return json_response(StatusCode::OK, json!({ "models": models }));
        }
        (&Method::POST, "/api/generate") => {}
        _ => {
            return error(
                StatusCode::NOT_FOUND,
                "Use /api/tags, /api/version, or /api/generate.",
            );
        }
    }
    let Some(route) = route else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Select an answer model in OpenKeet Settings.",
        );
    };
    let body = tokio::time::timeout(
        Duration::from_secs(10),
        Limited::new(request.into_body(), MAX_REQUEST_BYTES).collect(),
    )
    .await;
    let body = match body {
        Ok(Ok(body)) => body.to_bytes(),
        Ok(Err(problem)) if problem.is::<LengthLimitError>() => {
            return error(StatusCode::PAYLOAD_TOO_LARGE, "Use at most 150,000 bytes.");
        }
        Ok(Err(_)) => return error(StatusCode::BAD_REQUEST, "Could not read the request."),
        Err(_) => {
            return error(
                StatusCode::REQUEST_TIMEOUT,
                "The request body took too long.",
            );
        }
    };
    let Ok(input) = serde_json::from_slice::<Generate>(&body) else {
        return error(
            StatusCode::BAD_REQUEST,
            "A JSON request with a text prompt is required.",
        );
    };
    let prompt = input.prompt.trim();
    if prompt.is_empty() || prompt.len() > MAX_QUESTION_BYTES {
        return error(
            StatusCode::BAD_REQUEST,
            "Enter a prompt within the question length limit.",
        );
    }
    if input.images.is_some_and(|images| !images.is_empty()) || input.audio.is_some() {
        return error(StatusCode::BAD_REQUEST, "The bridge accepts text only.");
    }
    if input
        .model
        .as_ref()
        .is_some_and(|model| model != &route.model)
    {
        return error(
            StatusCode::BAD_REQUEST,
            "Choose the model returned by /api/tags.",
        );
    }
    let Ok(slot) = active.try_acquire_owned() else {
        return error(
            StatusCode::TOO_MANY_REQUESTS,
            "Wait for the current answer.",
        );
    };
    let question = one_off(prompt.to_owned());
    if !input.stream {
        let _slot = slot;
        return match hub
            .answer(&route, &question, |_| {
                hub.selection().as_ref() == Some(&route)
            })
            .await
        {
            Ok(text) => json_response(StatusCode::OK, answer(&route.model, &text, true)),
            Err(problem) => error(StatusCode::BAD_GATEWAY, problem.message()),
        };
    }
    let (send, receive) = mpsc::channel(128);
    tokio::spawn(async move {
        let _slot = slot;
        let result = tokio::select! {
            _ = send.closed() => return,
            result = hub.answer(&route, &question, |text| {
                hub.selection().as_ref() == Some(&route)
                    && send.try_send(line(answer(&route.model, text, false))).is_ok()
            }) => result,
        };
        let final_line = match result {
            Ok(_) => answer(&route.model, "", true),
            Err(problem) => json!({ "error": problem.message() }),
        };
        let _ = send.send(line(final_line)).await;
    });
    let body = StreamBody::new(stream::unfold(receive, |mut receive| async move {
        let bytes = receive.recv().await?;
        Some((Ok(Frame::data(bytes)), receive))
    }))
    .boxed_unsync();
    response(StatusCode::OK, "application/x-ndjson", body)
}

fn answer(model: &str, text: &str, done: bool) -> Value {
    json!({ "model": model, "response": text, "done": done })
}

fn line(value: Value) -> Bytes {
    Bytes::from(format!("{value}\n"))
}
