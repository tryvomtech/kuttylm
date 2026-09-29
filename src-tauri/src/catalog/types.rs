use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogFile {
    pub name: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub sha256: Option<String>,
}

/// One model entry from the API (format version 1). Unknown fields are ignored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CatalogModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub hf_repo: String,
    #[serde(default = "default_revision")]
    pub revision: String,
    pub files: Vec<CatalogFile>,
    #[serde(default)]
    pub mmproj: Option<CatalogFile>,
    #[serde(default)]
    pub quantization: Option<String>,
    #[serde(default)]
    pub params_b: Option<f64>,
    #[serde(default)]
    pub context_length: Option<u64>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub trending: bool,
}

fn default_revision() -> String {
    "main".to_string()
}

impl CatalogModel {
    /// All files plus the vision file. The "fits" check uses this.
    pub fn total_size_bytes(&self) -> u64 {
        self.files
            .iter()
            .chain(self.mmproj.iter())
            .map(|f| f.size_bytes)
            .sum()
    }

    /// Checks the fields that are later used in folder names, file names and URLs.
    pub fn validate(&self) -> Result<(), String> {
        if !is_safe_name(&self.id) {
            return Err(format!("bad id {:?}", self.id));
        }
        if self.name.trim().is_empty() {
            return Err(format!("{}: empty name", self.id));
        }
        if !is_valid_repo(&self.hf_repo) {
            return Err(format!("{}: bad hf_repo {:?}", self.id, self.hf_repo));
        }
        if !is_safe_name(&self.revision) {
            return Err(format!("{}: bad revision {:?}", self.id, self.revision));
        }
        if self.files.is_empty() {
            return Err(format!("{}: no files", self.id));
        }
        for file in self.files.iter().chain(self.mmproj.iter()) {
            if !is_safe_name(&file.name) || !file.name.to_lowercase().ends_with(".gguf") {
                return Err(format!("{}: bad file name {:?}", self.id, file.name));
            }
            if file.size_bytes == 0 {
                return Err(format!("{}: file {} has size 0", self.id, file.name));
            }
            if let Some(sha) = &file.sha256 {
                if sha.len() != 64 || !sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(format!("{}: bad sha256 for {}", self.id, file.name));
                }
            }
        }
        Ok(())
    }
}

/// Letters, digits, `.`, `_`, `-`. No slashes, no `..`, not starting with `.`.
pub(crate) fn is_safe_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 200
        && !s.starts_with('.')
        && !s.contains("..")
        && s
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// `owner/repo`, each part a safe name.
fn is_valid_repo(s: &str) -> bool {
    match s.split_once('/') {
        Some((owner, repo)) => is_safe_name(owner) && is_safe_name(repo),
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> CatalogModel {
        serde_json::from_value(serde_json::json!({
            "id": "m1",
            "name": "Model 1",
            "hf_repo": "owner/repo",
            "files": [{ "name": "a.gguf", "size_bytes": 100 }],
            "mmproj": { "name": "mmproj.gguf", "size_bytes": 20 },
            "some_future_field": 42
        }))
        .unwrap()
    }

    #[test]
    fn defaults_and_unknown_fields() {
        let m = model();
        assert_eq!(m.revision, "main");
        assert!(!m.trending);
        assert!(m.tags.is_empty());
        assert!(m.validate().is_ok());
    }

    #[test]
    fn total_size_includes_mmproj() {
        assert_eq!(model().total_size_bytes(), 120);
    }

    #[test]
    fn rejects_unsafe_names() {
        let mut m = model();
        m.id = "../evil".into();
        assert!(m.validate().is_err());

        let mut m = model();
        m.files[0].name = "sub/dir.gguf".into();
        assert!(m.validate().is_err());

        let mut m = model();
        m.files[0].name = "readme.txt".into();
        assert!(m.validate().is_err());

        let mut m = model();
        m.hf_repo = "no-slash".into();
        assert!(m.validate().is_err());

        let mut m = model();
        m.revision = "main/../x".into();
        assert!(m.validate().is_err());
    }

    #[test]
    fn rejects_bad_sha256() {
        let mut m = model();
        m.files[0].sha256 = Some("abc".into());
        assert!(m.validate().is_err());
    }
}
