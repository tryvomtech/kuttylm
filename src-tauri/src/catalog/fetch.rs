use std::time::Duration;

use reqwest::{header, StatusCode};

use crate::config;

/// The list is small. Anything bigger is not a real model list.
const MAX_BODY_BYTES: usize = 5 * 1024 * 1024;

pub enum FetchOutcome {
    Fresh { body: String, etag: Option<String> },
    /// HTTP 304: the cached list is still current.
    NotModified,
}

/// Downloads the model list. `etag` comes from the cache, if any.
pub async fn fetch(url: &str, etag: Option<&str>) -> Result<FetchOutcome, String> {
    let url = crate::net::check_url(url)?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config::MODELS_API_TIMEOUT_SECS))
        .user_agent(config::user_agent())
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let mut request = client.get(url).header(header::ACCEPT, "application/json");
    if let Some(tag) = etag {
        request = request.header(header::IF_NONE_MATCH, tag);
    }

    let response = request.send().await.map_err(|e| format!("request failed: {e}"))?;
    let status = response.status();
    if status == StatusCode::NOT_MODIFIED {
        return Ok(FetchOutcome::NotModified);
    }
    if !status.is_success() {
        return Err(format!("server answered {status}"));
    }
    if response
        .content_length()
        .is_some_and(|len| len as usize > MAX_BODY_BYTES)
    {
        return Err("response is too big".into());
    }

    let etag = response
        .headers()
        .get(header::ETAG)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = response.bytes().await.map_err(|e| format!("read failed: {e}"))?;
    if bytes.len() > MAX_BODY_BYTES {
        return Err("response is too big".into());
    }
    let body = String::from_utf8(bytes.to_vec()).map_err(|_| "response is not text".to_string())?;
    Ok(FetchOutcome::Fresh { body, etag })
}
