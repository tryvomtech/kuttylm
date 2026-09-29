//! Shared network rules.

use reqwest::Url;

/// HTTPS only. Plain HTTP is allowed for this computer only, to test with a local server.
pub fn check_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw.trim()).map_err(|_| format!("bad URL {raw:?}"))?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    match url.scheme() {
        "https" => Ok(url),
        "http" if local => Ok(url),
        _ => Err(format!("URL must use https: {raw:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_rules() {
        assert!(check_url("https://api.example.com/models").is_ok());
        assert!(check_url("http://127.0.0.1:8000/models.json").is_ok());
        assert!(check_url("http://localhost:8000/models.json").is_ok());
        assert!(check_url("http://api.example.com/models").is_err());
        assert!(check_url("file:///etc/passwd").is_err());
        assert!(check_url("not a url").is_err());
    }
}
