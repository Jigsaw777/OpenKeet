//! Native, public-client Sign in with ChatGPT flow. All credentials stay in Rust.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Url};
use ring::signature::{RSA_PKCS1_2048_8192_SHA256, RsaPublicKeyComponents};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    time::timeout,
};
use uuid::Uuid;

use crate::{platform::open_browser, providers::json_body};

const ISSUER: &str = "https://auth.openai.com";
const AUTHORIZE: &str = "https://auth.openai.com/api/accounts/authorize";
const TOKEN: &str = "https://auth.openai.com/api/accounts/oauth/token";
const JWKS: &str = "https://auth.openai.com/.well-known/jwks.json";
const REVOKE: &str = "https://auth.openai.com/api/accounts/oauth/revoke";
const RESOURCE: &str = "https://api.openai.com/v1";
const SCOPE: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct ChatGpt {
    registration: Option<Registration>,
    tokens: Option<Tokens>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Registration {
    client_id: String,
    subject: String,
    email: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct Tokens {
    access: String,
    refresh: String,
    id_token: Option<String>,
    scope: String,
    expiry: u64,
}

#[derive(Deserialize)]
struct Grant {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    scope: Option<String>,
    expires_in: u64,
    token_type: String,
}

impl ChatGpt {
    pub fn registered(&self) -> bool {
        self.registration.is_some()
    }

    pub fn account(&self) -> Option<&str> {
        self.tokens.as_ref()?;
        let registration = self.registration.as_ref()?;
        Some(registration.email.as_deref().unwrap_or("ChatGPT account"))
    }

    pub fn access(&self) -> Option<&str> {
        self.tokens
            .as_ref()
            .filter(|token| token.expiry > now().saturating_add(60) && plan_scope(&token.scope))
            .map(|token| token.access.as_str())
    }

    pub fn clear(&mut self) {
        self.tokens = None;
    }

    pub async fn renew(&self, client: &Client) -> Result<Self, String> {
        let registration = self
            .registration
            .as_ref()
            .ok_or("Continue with ChatGPT first.")?;
        let old = self.tokens.as_ref().ok_or("Continue with ChatGPT first.")?;
        let grant = token_request(
            client,
            &[
                ("grant_type", "refresh_token"),
                ("client_id", &registration.client_id),
                ("refresh_token", &old.refresh),
                ("resource", RESOURCE),
            ],
        )
        .await?;
        let tokens = token_set(grant, Some(old))?;
        Ok(Self {
            registration: self.registration.clone(),
            tokens: Some(tokens),
        })
    }

    pub async fn revoke(&self, client: &Client) -> Result<(), String> {
        if let (Some(registration), Some(token)) = (&self.registration, &self.tokens) {
            let result = client
                .post(REVOKE)
                .form(&[
                    ("token", token.refresh.as_str()),
                    ("token_type_hint", "refresh_token"),
                    ("client_id", registration.client_id.as_str()),
                ])
                .send()
                .await;
            if !result.is_ok_and(|response| response.status().is_success()) {
                return Err("Signed out on this device. Check connected apps in ChatGPT to revoke account access.".into());
            }
        }
        Ok(())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn plan_scope(scope: &str) -> bool {
    ["resource.invoke", "chatgpt.tokens.use.direct"]
        .iter()
        .all(|required| {
            scope
                .split_ascii_whitespace()
                .any(|scope| scope == *required)
        })
}

fn token_set(grant: Grant, previous: Option<&Tokens>) -> Result<Tokens, String> {
    let scope = grant
        .scope
        .or_else(|| previous.map(|old| old.scope.clone()))
        .unwrap_or_default();
    if !plan_scope(&scope) {
        return Err(
            "ChatGPT did not grant plan usage. Reconnect and allow AI requests for this app."
                .into(),
        );
    }
    if !grant.token_type.eq_ignore_ascii_case("Bearer")
        || grant.access_token.is_empty()
        || grant.expires_in == 0
    {
        return Err("ChatGPT returned an invalid authorization response.".into());
    }
    let refresh = grant
        .refresh_token
        .or_else(|| previous.map(|old| old.refresh.clone()))
        .filter(|token| !token.is_empty())
        .ok_or("ChatGPT did not issue a refresh token.")?;
    Ok(Tokens {
        access: grant.access_token,
        refresh,
        id_token: grant
            .id_token
            .or_else(|| previous.and_then(|old| old.id_token.clone())),
        scope,
        expiry: now().saturating_add(grant.expires_in),
    })
}

async fn token_request(client: &Client, form: &[(&str, &str)]) -> Result<Grant, String> {
    let response = client
        .post(TOKEN)
        .form(form)
        .send()
        .await
        .map_err(|_| "Could not reach ChatGPT sign-in.")?;
    if !response.status().is_success() {
        return Err("ChatGPT could not authorize this connection. Try signing in again.".into());
    }
    json_body(response)
        .await
        .map_err(|_| "ChatGPT returned an invalid token response.".into())
}

pub async fn sign_in(
    client: &Client,
    host: &str,
    previous: &ChatGpt,
    different_account: bool,
) -> Result<ChatGpt, String> {
    let returning = (!different_account)
        .then_some(previous.registration.as_ref())
        .flatten();
    let known_client = returning.map(|account| account.client_id.as_str());
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .map_err(|_| "Could not start the local sign-in callback.")?;
    let port = listener
        .local_addr()
        .map_err(|_| "Could not open the sign-in callback.")?
        .port();
    let redirect = format!("http://127.0.0.1:{port}/chatgpt/callback");
    let state = Uuid::new_v4().to_string();
    let nonce = Uuid::new_v4().to_string();
    let verifier = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let mut url = Url::parse(AUTHORIZE).expect("constant authorization endpoint");
    {
        let mut query = url.query_pairs_mut();
        query.extend_pairs([
            ("client_id", known_client.unwrap_or("dynamic_agent_client")),
            ("ext_agent_host_id", host),
            ("response_type", "code"),
            ("redirect_uri", &redirect),
            ("scope", SCOPE),
            ("resource", RESOURCE),
            ("state", &state),
            ("nonce", &nonce),
            ("code_challenge_method", "S256"),
            ("code_challenge", &challenge),
        ]);
        if known_client.is_none() {
            query.append_pair("agent_name_hint", "Personal Copilot");
        }
    }
    open_browser(url.as_str())?;
    let callback = timeout(
        Duration::from_secs(180),
        receive_callback(listener, &state, known_client),
    )
    .await
    .map_err(|_| "ChatGPT sign-in expired. Start again in Providers.")??;
    let grant = token_request(
        client,
        &[
            ("grant_type", "authorization_code"),
            ("client_id", &callback.client_id),
            ("code", &callback.code),
            ("code_verifier", &verifier),
            ("redirect_uri", &redirect),
            ("resource", RESOURCE),
        ],
    )
    .await?;
    let tokens = token_set(grant, None)?;
    let claims = verify_identity(
        client,
        tokens
            .id_token
            .as_deref()
            .ok_or("ChatGPT did not supply a verifiable identity.")?,
        &callback.client_id,
        &nonce,
    )
    .await?;
    if returning.is_some_and(|account| account.subject != claims.sub) {
        return Err(
            "The browser returned a different account. Use Add another account to switch.".into(),
        );
    }
    Ok(ChatGpt {
        registration: Some(Registration {
            client_id: callback.client_id,
            subject: claims.sub,
            email: claims.email,
        }),
        tokens: Some(tokens),
    })
}

struct Callback {
    client_id: String,
    code: String,
}

fn parse_callback(
    request: &str,
    state: &str,
    known_client: Option<&str>,
) -> Result<Option<Callback>, String> {
    let Some(line) = request.lines().next() else {
        return Ok(None);
    };
    let mut parts = line.split_ascii_whitespace();
    let (Some("GET"), Some(target)) = (parts.next(), parts.next()) else {
        return Ok(None);
    };
    if !target.starts_with("/chatgpt/callback?") {
        return Ok(None);
    }
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Ok(None);
    };
    let mut fields = std::collections::BTreeMap::new();
    for (key, value) in url.query_pairs() {
        if fields.insert(key, value).is_some() {
            return Ok(None);
        }
    }
    if fields.get("state").map(|s| s.as_ref()) != Some(state) {
        return Ok(None);
    }
    if fields.contains_key("error") {
        return Err("ChatGPT sign-in was declined or did not complete.".into());
    }
    let client_id = fields
        .get("client_id")
        .map(|s| s.as_ref())
        .or(known_client)
        .filter(|id| {
            !id.is_empty()
                && *id != "dynamic_agent_client"
                && id.len() <= 256
                && id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
        .ok_or("ChatGPT did not return a valid client registration.")?;
    if known_client.is_some_and(|expected| client_id != expected) {
        return Err("ChatGPT returned an unexpected client registration.".into());
    }
    let code = fields
        .get("code")
        .filter(|code| !code.is_empty() && code.len() <= 4_096)
        .ok_or("ChatGPT did not return an authorization code.")?;
    Ok(Some(Callback {
        client_id: client_id.into(),
        code: code.to_string(),
    }))
}

async fn read_request(stream: &mut TcpStream) -> std::io::Result<String> {
    let mut request = Vec::with_capacity(1024);
    let mut block = [0; 1024];
    while request.len() < 8192 {
        let length = stream.read(&mut block).await?;
        if length == 0 {
            break;
        }
        request.extend_from_slice(&block[..length]);
        if request.windows(2).any(|part| part == b"\r\n") {
            return String::from_utf8(request).map_err(std::io::Error::other);
        }
    }
    Err(std::io::Error::other("invalid callback request"))
}

async fn receive_callback(
    listener: TcpListener,
    state: &str,
    known_client: Option<&str>,
) -> Result<Callback, String> {
    loop {
        let (mut stream, _) = listener
            .accept()
            .await
            .map_err(|_| "The local sign-in callback closed.")?;
        let Ok(Ok(request)) = timeout(Duration::from_secs(2), read_request(&mut stream)).await
        else {
            continue;
        };
        let parsed = parse_callback(&request, state, known_client);
        let (status, body) = match &parsed {
            Ok(Some(_)) => (
                "200 OK",
                "Authorization response received. Return to Personal Copilot.",
            ),
            Ok(None) => (
                "400 Bad Request",
                "This is not the current Personal Copilot sign-in request.",
            ),
            Err(_) => (
                "400 Bad Request",
                "Sign-in did not complete. Return to Personal Copilot.",
            ),
        };
        let reply = format!(
            "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = timeout(Duration::from_secs(1), stream.write_all(reply.as_bytes())).await;
        if let Some(callback) = parsed? {
            return Ok(callback);
        }
    }
}

#[derive(Deserialize)]
struct JwtHeader {
    alg: String,
    kid: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    aud: Audience,
    exp: u64,
    nbf: Option<u64>,
    nonce: String,
    azp: Option<String>,
    email: Option<String>,
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<Jwk>,
}

#[derive(Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: Option<String>,
    e: Option<String>,
}

fn part<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, &'static str> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| "Invalid ChatGPT identity token.")?;
    serde_json::from_slice(&bytes).map_err(|_| "Invalid ChatGPT identity token.")
}

async fn verify_identity(
    client: &Client,
    token: &str,
    client_id: &str,
    nonce: &str,
) -> Result<Claims, String> {
    let invalid = "Invalid ChatGPT identity token.";
    if token.len() > 32_768 {
        return Err(invalid.into());
    }
    let (signed, signature) = token.rsplit_once('.').ok_or(invalid)?;
    let (header, claims) = signed.split_once('.').ok_or(invalid)?;
    let header: JwtHeader = part(header)?;
    if header.alg != "RS256" || header.kid.is_empty() {
        return Err(invalid.into());
    }
    let response = client
        .get(JWKS)
        .send()
        .await
        .map_err(|_| "Could not verify the ChatGPT identity.")?;
    let keys: Jwks = json_body(response)
        .await
        .map_err(|_| "Could not verify the ChatGPT identity.")?;
    let key = keys
        .keys
        .into_iter()
        .find(|key| key.kid == header.kid && key.kty == "RSA")
        .ok_or(invalid)?;
    let n = URL_SAFE_NO_PAD
        .decode(key.n.ok_or(invalid)?)
        .map_err(|_| invalid)?;
    let e = URL_SAFE_NO_PAD
        .decode(key.e.ok_or(invalid)?)
        .map_err(|_| invalid)?;
    let signature = URL_SAFE_NO_PAD.decode(signature).map_err(|_| invalid)?;
    RsaPublicKeyComponents { n: &n, e: &e }
        .verify(&RSA_PKCS1_2048_8192_SHA256, signed.as_bytes(), &signature)
        .map_err(|_| invalid)?;
    let claims: Claims = part(claims)?;
    let audience_ok = match &claims.aud {
        Audience::Single(audience) => audience == client_id,
        Audience::Multiple(audiences) => {
            audiences.iter().any(|audience| audience == client_id)
                && (audiences.len() == 1 || claims.azp.as_deref() == Some(client_id))
        }
    };
    if claims.iss != ISSUER
        || !audience_ok
        || claims.nonce != nonce
        || claims.sub.is_empty()
        || claims.exp <= now()
        || claims
            .nbf
            .is_some_and(|date| date > now().saturating_add(60))
        || claims.azp.as_ref().is_some_and(|party| party != client_id)
    {
        return Err(invalid.into());
    }
    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_binds_state_and_registered_client() {
        assert!(
            parse_callback(
                "GET /chatgpt/callback?state=bad&code=x&client_id=oaiapp_new HTTP/1.1",
                "nonce",
                None
            )
            .unwrap()
            .is_none()
        );
        assert!(
            parse_callback(
                "GET /chatgpt/callback?state=nonce&state=bad HTTP/1.1",
                "nonce",
                None
            )
            .unwrap()
            .is_none()
        );
        assert!(
            parse_callback(
                "GET /chatgpt/callback?state=nonce&error=access_denied HTTP/1.1",
                "nonce",
                None
            )
            .is_err()
        );
        assert!(
            parse_callback(
                "GET /chatgpt/callback?state=nonce&code=x&client_id=oaiapp_new HTTP/1.1",
                "nonce",
                Some("oaiapp_old")
            )
            .is_err()
        );
        let callback = parse_callback(
            "GET /chatgpt/callback?state=nonce&code=a%2Bb&client_id=oaiapp_new HTTP/1.1",
            "nonce",
            None,
        )
        .unwrap()
        .unwrap();
        assert_eq!(callback.code, "a+b");
        assert_eq!(callback.client_id, "oaiapp_new");
    }

    #[test]
    fn identity_scope_cannot_start_plan_requests() {
        assert!(!plan_scope("openid profile email offline_access"));
        assert!(plan_scope(
            "openid chatgpt.tokens.use.direct resource.invoke"
        ));
    }
}
