use serde_json::Value;

use super::types::CatalogModel;
use crate::config::MODELS_API_VERSION;

#[derive(Debug)]
pub struct ParsedCatalog {
    pub models: Vec<CatalogModel>,
    /// Entries that were broken or unsafe and left out.
    pub skipped: usize,
}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    NotJson,
    /// The API sends a newer format than this app knows.
    TooNew(u64),
    BadFormat(&'static str),
}

/// Parses `{ "version": 1, "models": [...] }`.
/// One bad model entry is skipped; the rest of the list is kept.
pub fn parse(body: &str) -> Result<ParsedCatalog, ParseError> {
    let root: Value = serde_json::from_str(body).map_err(|_| ParseError::NotJson)?;

    let version = root
        .get("version")
        .and_then(Value::as_u64)
        .ok_or(ParseError::BadFormat("missing version"))?;
    if version > MODELS_API_VERSION {
        return Err(ParseError::TooNew(version));
    }

    let entries = root
        .get("models")
        .and_then(Value::as_array)
        .ok_or(ParseError::BadFormat("missing models list"))?;

    let mut models: Vec<CatalogModel> = Vec::with_capacity(entries.len());
    let mut skipped = 0;
    for entry in entries {
        let model = match serde_json::from_value::<CatalogModel>(entry.clone()) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("[catalog] skipped entry: {e}");
                skipped += 1;
                continue;
            }
        };
        if let Err(reason) = model.validate() {
            eprintln!("[catalog] skipped entry: {reason}");
            skipped += 1;
            continue;
        }
        if models.iter().any(|m| m.id == model.id) {
            eprintln!("[catalog] skipped duplicate id {}", model.id);
            skipped += 1;
            continue;
        }
        models.push(model);
    }

    Ok(ParsedCatalog { models, skipped })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOCK: &str = include_str!("../../resources/models.mock.json");

    #[test]
    fn mock_file_is_valid() {
        let parsed = parse(MOCK).unwrap();
        assert_eq!(parsed.skipped, 0);
        assert!(parsed.models.len() >= 5);
    }

    #[test]
    fn skips_bad_entries_and_keeps_good_ones() {
        let body = r#"{
          "version": 1,
          "models": [
            { "id": "good", "name": "Good", "hf_repo": "a/b",
              "files": [{ "name": "g.gguf", "size_bytes": 1 }] },
            { "id": "no-files", "name": "Bad", "hf_repo": "a/b", "files": [] },
            { "name": "missing id" },
            { "id": "good", "name": "Duplicate", "hf_repo": "a/b",
              "files": [{ "name": "g.gguf", "size_bytes": 1 }] }
          ]
        }"#;
        let parsed = parse(body).unwrap();
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.models[0].id, "good");
        assert_eq!(parsed.skipped, 3);
    }

    #[test]
    fn newer_version_is_reported() {
        assert_eq!(
            parse(r#"{ "version": 2, "models": [] }"#).unwrap_err(),
            ParseError::TooNew(2)
        );
    }

    #[test]
    fn bad_bodies() {
        assert_eq!(parse("<html>").unwrap_err(), ParseError::NotJson);
        assert!(matches!(parse("[]").unwrap_err(), ParseError::BadFormat(_)));
        assert!(matches!(
            parse(r#"{ "version": 1 }"#).unwrap_err(),
            ParseError::BadFormat(_)
        ));
    }
}
