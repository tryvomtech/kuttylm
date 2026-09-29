//! Turns a Hugging Face file list into downloadable model versions ("variants").
//!
//! - One variant per GGUF file, or per set of split files (`-00001-of-00003.gguf`).
//! - `mmproj` files (vision) are not variants; the best one is added to every variant.
//! - Files in subfolders and imatrix files are skipped.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::catalog::types::{is_safe_name, CatalogFile, CatalogModel};

/// One entry of `GET /api/models/{repo}/tree/{revision}`.
#[derive(Debug, Clone, Deserialize)]
pub struct TreeEntry {
    #[serde(rename = "type")]
    pub kind: String,
    pub path: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub lfs: Option<Lfs>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Lfs {
    /// The sha256 of the file.
    pub oid: String,
}

/// Facts about the repository that every variant shares.
#[derive(Debug, Clone, Default)]
pub struct RepoFacts {
    pub repo: String,
    pub revision: String,
    pub context_length: Option<u64>,
    pub params_b: Option<f64>,
    pub license: Option<String>,
}

fn file(entry: &TreeEntry) -> CatalogFile {
    CatalogFile {
        name: entry.path.clone(),
        size_bytes: entry.size,
        sha256: entry.lfs.as_ref().map(|l| l.oid.to_lowercase()).filter(|s| s.len() == 64),
    }
}

/// `model-Q4_K_M-00002-of-00003.gguf` → `Some(("model-Q4_K_M", 2, 3))`.
fn split_part(name: &str) -> Option<(&str, u32, u32)> {
    let stem = name.strip_suffix(".gguf")?;
    let (rest, total) = stem.rsplit_once("-of-")?;
    let (prefix, part) = rest.rsplit_once('-')?;
    if part.len() != 5 || total.len() != 5 {
        return None;
    }
    Some((prefix, part.parse().ok()?, total.parse().ok()?))
}

/// The quantization written in a file name, e.g. `Q4_K_M`, `IQ3_XS`, `F16`.
pub fn quant_of(stem: &str) -> Option<String> {
    let last = stem.rsplit(['-', '.']).next()?.to_uppercase();
    let looks_like_quant = |s: &str| {
        let rest = s
            .strip_prefix("IQ")
            .or_else(|| s.strip_prefix("Q"))
            .or_else(|| s.strip_prefix("BF"))
            .or_else(|| s.strip_prefix("F"))
            .or_else(|| s.strip_prefix("MXFP"));
        rest.is_some_and(|r| r.starts_with(|c: char| c.is_ascii_digit()))
    };
    looks_like_quant(&last).then_some(last)
}

/// Picks the vision file: Q8_0 is small and good; then F16, BF16, then any.
fn best_mmproj(entries: &[&TreeEntry]) -> Option<CatalogFile> {
    let rank = |name: &str| {
        let upper = name.to_uppercase();
        ["Q8_0", "F16", "BF16", "F32"].iter().position(|q| upper.contains(q)).unwrap_or(9)
    };
    entries.iter().min_by_key(|e| (rank(&e.path), e.size)).map(|e| file(e))
}

/// Lower-case, safe id: `hf-owner--repo--file-stem` (max 200 characters).
fn variant_id(repo: &str, stem: &str) -> String {
    let raw = format!("hf-{}--{}", repo.replace('/', "--"), stem);
    let mut id: String = raw
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c.to_ascii_lowercase() } else { '-' })
        .collect();
    id.truncate(200);
    id.trim_end_matches(['.', '-']).to_string()
}

/// "bartowski/Llama-3.2-3B-Instruct-GGUF" → "Llama-3.2-3B-Instruct".
pub fn short_repo_name(repo: &str) -> String {
    let name = repo.rsplit('/').next().unwrap_or(repo);
    for suffix in ["-GGUF", "-gguf", "_GGUF", ".gguf", "-Gguf"] {
        if let Some(s) = name.strip_suffix(suffix) {
            return s.to_string();
        }
    }
    name.to_string()
}

pub fn build_variants(facts: &RepoFacts, tree: &[TreeEntry]) -> Vec<CatalogModel> {
    let ggufs: Vec<&TreeEntry> = tree
        .iter()
        .filter(|e| e.kind == "file" && e.path.to_lowercase().ends_with(".gguf"))
        // Subfolders are not supported yet (file names must not contain '/').
        .filter(|e| !e.path.contains('/') && e.size > 0)
        .filter(|e| !e.path.to_lowercase().contains("imatrix"))
        .collect();

    let (mmproj, models): (Vec<&TreeEntry>, Vec<&TreeEntry>) =
        ggufs.into_iter().partition(|e| e.path.to_lowercase().contains("mmproj"));
    let vision = best_mmproj(&mmproj);

    // Group split parts by their common prefix; single files stand alone.
    let mut groups: BTreeMap<String, Vec<(u32, u32, &TreeEntry)>> = BTreeMap::new();
    for e in models {
        match split_part(&e.path) {
            Some((prefix, part, total)) => groups.entry(prefix.to_string()).or_default().push((part, total, e)),
            None => groups
                .entry(e.path.trim_end_matches(".gguf").to_string())
                .or_default()
                .push((1, 1, e)),
        }
    }

    let short = short_repo_name(&facts.repo);
    let mut variants: Vec<CatalogModel> = groups
        .into_iter()
        .filter_map(|(stem, mut parts)| {
            parts.sort_by_key(|p| p.0);
            let total = parts[0].1;
            // Every part must be there, numbered 1..=total.
            let complete = parts.len() as u32 == total && parts.iter().enumerate().all(|(i, p)| p.0 == i as u32 + 1);
            if !complete {
                return None;
            }
            let quant = quant_of(&stem);
            if quant.as_deref().is_some_and(|q| UNSUPPORTED.contains(&q)) {
                return None;
            }
            let model = CatalogModel {
                id: variant_id(&facts.repo, &stem),
                name: match &quant {
                    Some(q) => format!("{short} · {q}"),
                    None => format!("{short} · {stem}"),
                },
                description: None,
                hf_repo: facts.repo.clone(),
                revision: facts.revision.clone(),
                files: parts.iter().map(|p| file(p.2)).collect(),
                mmproj: vision.clone(),
                quantization: quant,
                params_b: facts.params_b,
                context_length: facts.context_length,
                license: facts.license.clone(),
                tags: vec!["huggingface".into()],
                trending: false,
            };
            // The same safety rules as the curated list.
            model.validate().ok().map(|_| model)
        })
        .filter(|m| is_safe_name(&m.id))
        .collect();

    variants.sort_by_key(|m| m.total_size_bytes());
    variants
}

/// Old ARM-only formats. Current llama.cpp cannot load them.
const UNSUPPORTED: [&str; 3] = ["Q4_0_4_4", "Q4_0_4_8", "Q4_0_8_8"];

/// Common good choices first: best quality per size for most people.
const PREFERRED: [&str; 12] = [
    "Q4_K_M", "Q4_K_L", "Q4_K_S", "IQ4_XS", "Q4_0", "Q5_K_M", "Q3_K_M", "Q3_K_L", "IQ3_M", "Q6_K", "Q8_0", "Q2_K",
];

/// Index of the variant to recommend, among those `fits` says fit well.
pub fn recommend(variants: &[CatalogModel], fits: impl Fn(&CatalogModel) -> bool) -> Option<usize> {
    for q in PREFERRED {
        if let Some(i) = variants.iter().position(|v| v.quantization.as_deref() == Some(q) && fits(v)) {
            return Some(i);
        }
    }
    // Otherwise the biggest one that fits.
    variants.iter().rposition(|v| fits(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str, size: u64) -> TreeEntry {
        TreeEntry {
            kind: "file".into(),
            path: path.into(),
            size,
            lfs: Some(Lfs { oid: format!("{:0>64}", size) }),
        }
    }

    fn facts() -> RepoFacts {
        RepoFacts {
            repo: "Owner/Cool-Model-7B-GGUF".into(),
            revision: "abc123".into(),
            context_length: Some(32768),
            params_b: Some(7.6),
            license: Some("apache-2.0".into()),
        }
    }

    #[test]
    fn quant_names() {
        assert_eq!(quant_of("Llama-3.2-3B-Instruct-Q4_K_M").as_deref(), Some("Q4_K_M"));
        assert_eq!(quant_of("qwen2.5-0.5b-instruct-q4_k_m").as_deref(), Some("Q4_K_M"));
        assert_eq!(quant_of("model.IQ3_XS").as_deref(), Some("IQ3_XS"));
        assert_eq!(quant_of("Phi-3-mini-4k-instruct-q4").as_deref(), Some("Q4"));
        assert_eq!(quant_of("Qwen3-8B-UD-Q4_K_XL").as_deref(), Some("Q4_K_XL"));
        assert_eq!(quant_of("model-BF16").as_deref(), Some("BF16"));
        assert_eq!(quant_of("model-f16").as_deref(), Some("F16"));
        assert_eq!(quant_of("ggml-model"), None);
        assert_eq!(quant_of("Qwen"), None);
    }

    #[test]
    fn split_names() {
        assert_eq!(split_part("m-Q8_0-00002-of-00003.gguf"), Some(("m-Q8_0", 2, 3)));
        assert_eq!(split_part("m-Q8_0.gguf"), None);
    }

    #[test]
    fn builds_variants() {
        let tree = vec![
            entry("README.md", 10),
            entry("Cool-Model-7B-Q4_K_M.gguf", 4_000),
            entry("Cool-Model-7B-Q2_K.gguf", 2_000),
            entry("Cool-Model-7B-Q4_0_4_4.gguf", 3_900),
            entry("Cool-Model-7B-Q8_0-00001-of-00002.gguf", 5_000),
            entry("Cool-Model-7B-Q8_0-00002-of-00002.gguf", 3_000),
            // Incomplete split set: skipped.
            entry("Cool-Model-7B-F16-00001-of-00003.gguf", 9_000),
            entry("mmproj-Cool-F16.gguf", 600),
            entry("mmproj-Cool-Q8_0.gguf", 300),
            entry("imatrix.gguf", 50),
            entry("Q6_K/Cool-Model-7B-Q6_K-00001-of-00002.gguf", 7_000),
        ];
        let v = build_variants(&facts(), &tree);
        let names: Vec<_> = v.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["Cool-Model-7B · Q2_K", "Cool-Model-7B · Q4_K_M", "Cool-Model-7B · Q8_0"]);

        let q8 = &v[2];
        assert_eq!(q8.files.len(), 2);
        assert_eq!(q8.files[0].name, "Cool-Model-7B-Q8_0-00001-of-00002.gguf");
        assert_eq!(q8.id, "hf-owner--cool-model-7b-gguf--cool-model-7b-q8_0");
        assert_eq!(q8.revision, "abc123");
        assert_eq!(q8.mmproj.as_ref().unwrap().name, "mmproj-Cool-Q8_0.gguf");
        assert_eq!(q8.total_size_bytes(), 8_300);
        assert!(q8.files[0].sha256.is_some());
        assert!(v.iter().all(|m| m.validate().is_ok()));
    }

    #[test]
    fn recommends_q4_k_m_when_it_fits() {
        let v = build_variants(
            &facts(),
            &[entry("M-Q2_K.gguf", 2), entry("M-Q4_K_M.gguf", 4), entry("M-Q8_0.gguf", 8)],
        );
        assert_eq!(v[recommend(&v, |_| true).unwrap()].quantization.as_deref(), Some("Q4_K_M"));
        // Only Q2_K fits.
        assert_eq!(v[recommend(&v, |m| m.total_size_bytes() < 3).unwrap()].quantization.as_deref(), Some("Q2_K"));
        assert_eq!(recommend(&v, |_| false), None);
    }

    #[test]
    fn short_names() {
        assert_eq!(short_repo_name("bartowski/Llama-3.2-3B-Instruct-GGUF"), "Llama-3.2-3B-Instruct");
        assert_eq!(short_repo_name("x/plain"), "plain");
    }
}
