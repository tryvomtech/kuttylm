//! Calls to the public Hugging Face API (no login).

use std::time::Duration;

use reqwest::{header, StatusCode};
use serde::{Deserialize, Serialize};

use super::variants::{RepoFacts, TreeEntry};
use crate::catalog::types::is_safe_name;
use crate::config;
use crate::error::{AppError, AppResult};

pub const PAGE_SIZE: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortBy {
    #[default]
    Downloads,
    Likes,
    Updated,
    Trending,
}

impl SortBy {
    fn param(self) -> &'static str {
        match self {
            SortBy::Downloads => "downloads",
            SortBy::Likes => "likes",
            SortBy::Updated => "lastModified",
            SortBy::Trending => "trendingScore",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SizeFilter {
    #[default]
    Any,
    /// Under 1 billion parameters.
    Tiny,
    /// 1 to 4 billion.
    Small,
    /// 4 to 14 billion.
    Medium,
    /// 14 billion and more.
    Large,
}

impl SizeFilter {
    fn param(self) -> Option<&'static str> {
        match self {
            SizeFilter::Any => None,
            SizeFilter::Tiny => Some("min:0,max:1B"),
            SizeFilter::Small => Some("min:1B,max:4B"),
            SizeFilter::Medium => Some("min:4B,max:14B"),
            SizeFilter::Large => Some("min:14B"),
        }
    }
}

/// One search result.
#[derive(Debug, Clone, Serialize)]
pub struct HfRepo {
    pub id: String,
    pub downloads: u64,
    pub likes: u64,
    pub pipeline_tag: Option<String>,
    pub last_modified: Option<String>,
    pub license: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchPage {
    pub results: Vec<HfRepo>,
    /// Give this back to get the next page. `None` = no more results.
    pub next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawRepo {
    id: String,
    #[serde(default)]
    downloads: u64,
    #[serde(default)]
    likes: u64,
    #[serde(default)]
    pipeline_tag: Option<String>,
    #[serde(default, rename = "lastModified")]
    last_modified: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    /// `false`, or "auto" / "manual" when a login is needed.
    #[serde(default)]
    gated: serde_json::Value,
    #[serde(default)]
    private: bool,
    #[serde(default)]
    sha: Option<String>,
    #[serde(default)]
    gguf: Option<RawGguf>,
}

#[derive(Debug, Deserialize)]
struct RawGguf {
    #[serde(default)]
    total: Option<u64>,
    #[serde(default)]
    context_length: Option<u64>,
}

/// Models for search and similarity cannot chat.
const NOT_CHAT: [&str; 3] = ["feature-extraction", "sentence-similarity", "text-ranking"];

fn is_open(raw: &RawRepo) -> bool {
    !raw.private && matches!(raw.gated, serde_json::Value::Bool(false) | serde_json::Value::Null)
}

fn license_of(tags: &[String]) -> Option<String> {
    tags.iter().find_map(|t| t.strip_prefix("license:")).map(str::to_string)
}

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(config::user_agent())
        .build()
        .map_err(|e| AppError::Internal(format!("http client: {e}")))
}

fn base_url() -> AppResult<String> {
    let base = config::hf_base_url();
    crate::net::check_url(&base).map_err(AppError::Internal)?;
    Ok(base.trim_end_matches('/').to_string())
}

fn offline(e: reqwest::Error) -> AppError {
    eprintln!("[hf] request failed: {e}");
    AppError::HuggingFaceUnavailable
}

pub fn search_url(base: &str, query: &str, sort: SortBy, size: SizeFilter, cursor: Option<&str>) -> String {
    let mut params: Vec<(&str, String)> = vec![
        ("filter", "gguf".into()),
        ("sort", sort.param().into()),
        ("direction", "-1".into()),
        ("limit", PAGE_SIZE.to_string()),
    ];
    let query = query.trim();
    if !query.is_empty() {
        params.push(("search", query.into()));
    }
    if let Some(p) = size.param() {
        params.push(("num_parameters", p.into()));
    }
    if let Some(c) = cursor {
        params.push(("cursor", c.into()));
    }
    for field in ["gated", "downloads", "likes", "pipeline_tag", "lastModified", "tags"] {
        params.push(("expand[]", field.into()));
    }
    let query_string: String = params
        .iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{base}/api/models?{query_string}")
}

/// Percent-encoding for URL query values.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// `<https://…&cursor=abc>; rel="next"` → the cursor value.
pub fn next_cursor(link: &str) -> Option<String> {
    let next = link.split(',').find(|part| part.contains("rel=\"next\""))?;
    let url = next.split(['<', '>']).nth(1)?;
    let query = url.split_once('?')?.1;
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("cursor="))
        .map(|c| percent_decode(c))
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub async fn search(query: &str, sort: SortBy, size: SizeFilter, cursor: Option<&str>) -> AppResult<SearchPage> {
    if query.len() > 200 {
        return Err(AppError::InvalidSetting("The search text is too long.".into()));
    }
    let url = search_url(&base_url()?, query, sort, size, cursor);
    let response = client()?.get(url).send().await.map_err(offline)?;
    if !response.status().is_success() {
        eprintln!("[hf] search answered {}", response.status());
        return Err(AppError::HuggingFaceUnavailable);
    }
    let next = response
        .headers()
        .get(header::LINK)
        .and_then(|v| v.to_str().ok())
        .and_then(next_cursor);
    let raw: Vec<RawRepo> = response.json().await.map_err(offline)?;
    let results = raw
        .into_iter()
        .filter(is_open)
        .filter(|r| !r.pipeline_tag.as_deref().is_some_and(|p| NOT_CHAT.contains(&p)))
        .map(|r| HfRepo {
            license: license_of(&r.tags),
            id: r.id,
            downloads: r.downloads,
            likes: r.likes,
            pipeline_tag: r.pipeline_tag,
            last_modified: r.last_modified,
        })
        .collect();
    Ok(SearchPage { results, next_cursor: next })
}

/// `owner/name`, both parts safe for URLs and folder names.
pub fn check_repo(repo: &str) -> AppResult<()> {
    match repo.split_once('/') {
        Some((owner, name)) if is_safe_name(owner) && is_safe_name(name) => Ok(()),
        _ => Err(AppError::Internal(format!("bad repository name {repo:?}"))),
    }
}

/// Repository facts and its file list, pinned to the current commit.
pub async fn repo_files(repo: &str) -> AppResult<(RepoFacts, Vec<TreeEntry>)> {
    check_repo(repo)?;
    let base = base_url()?;
    let client = client()?;

    let info_url = format!("{base}/api/models/{repo}?expand[]=gated&expand[]=sha&expand[]=gguf&expand[]=tags");
    let response = client.get(info_url).send().await.map_err(offline)?;
    match response.status() {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => return Err(AppError::NeedsLogin),
        StatusCode::NOT_FOUND => return Err(AppError::FileNotFoundOnline),
        s if !s.is_success() => return Err(AppError::HuggingFaceUnavailable),
        _ => {}
    }
    let info: RawRepo = response.json().await.map_err(offline)?;
    if !is_open(&info) {
        return Err(AppError::NeedsLogin);
    }
    let revision = info
        .sha
        .clone()
        .filter(|s| is_safe_name(s))
        .ok_or(AppError::HuggingFaceUnavailable)?;

    let tree_url = format!("{base}/api/models/{repo}/tree/{revision}");
    let response = client.get(tree_url).send().await.map_err(offline)?;
    if !response.status().is_success() {
        return Err(AppError::HuggingFaceUnavailable);
    }
    let tree: Vec<TreeEntry> = response.json().await.map_err(offline)?;

    let facts = RepoFacts {
        repo: info.id.clone(),
        revision,
        context_length: info.gguf.as_ref().and_then(|g| g.context_length),
        params_b: info
            .gguf
            .as_ref()
            .and_then(|g| g.total)
            .map(|t| (t as f64 / 1e8).round() / 10.0),
        license: license_of(&info.tags),
    };
    Ok((facts, tree))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_search_url() {
        let url = search_url("https://huggingface.co", " llama 3 ", SortBy::Likes, SizeFilter::Small, Some("a b"));
        assert!(url.starts_with("https://huggingface.co/api/models?filter=gguf&sort=likes&direction=-1&limit=20"));
        assert!(url.contains("&search=llama%203"));
        assert!(url.contains("&num_parameters=min%3A1B%2Cmax%3A4B"));
        assert!(url.contains("&cursor=a%20b"));
        assert!(url.contains("&expand%5B%5D=gated"));
    }

    #[test]
    fn reads_next_cursor() {
        let link = r#"<https://huggingface.co/api/models?filter=gguf&limit=20&cursor=eyJhIjoxfQ%3D%3D>; rel="next""#;
        assert_eq!(next_cursor(link).as_deref(), Some("eyJhIjoxfQ=="));
        assert_eq!(next_cursor(""), None);
    }

    #[test]
    fn gated_and_private_are_hidden() {
        let repo = |gated: serde_json::Value, private: bool| RawRepo {
            id: "a/b".into(),
            downloads: 0,
            likes: 0,
            pipeline_tag: None,
            last_modified: None,
            tags: vec![],
            gated,
            private,
            sha: None,
            gguf: None,
        };
        assert!(is_open(&repo(serde_json::json!(false), false)));
        assert!(is_open(&repo(serde_json::Value::Null, false)));
        assert!(!is_open(&repo(serde_json::json!("manual"), false)));
        assert!(!is_open(&repo(serde_json::json!(false), true)));
    }

    #[test]
    fn repo_names() {
        assert!(check_repo("bartowski/Llama-3.2-3B-Instruct-GGUF").is_ok());
        assert!(check_repo("../etc").is_err());
        assert!(check_repo("a/b/c").is_err());
        assert!(check_repo("noslash").is_err());
    }
}
