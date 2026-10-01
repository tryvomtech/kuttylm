//! Documents on disk: `<data>/kuttylm/documents/<id>/`
//! - `meta.json`: name, size, state
//! - `chunks.json`: the text parts
//! - `vectors.bin`: one vector per part (little-endian f32)
//!
//! The original file is not kept. Only its text is.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::chunk::Chunk;
use super::parse::DocKind;
use crate::catalog::types::is_safe_name;
use crate::error::{AppError, AppResult};

const META: &str = "meta.json";
const CHUNKS: &str = "chunks.json";
const VECTORS: &str = "vectors.bin";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocStatus {
    Indexing,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocMeta {
    pub id: String,
    pub name: String,
    pub kind: DocKind,
    pub size_bytes: u64,
    /// PDF only: pages with text.
    #[serde(default)]
    pub pages: Option<u32>,
    #[serde(default)]
    pub chunks: usize,
    pub created_at: u64,
    pub status: DocStatus,
    #[serde(default)]
    pub error: Option<String>,
    /// Vectors from another model cannot be compared, so this is kept.
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub dim: usize,
}

pub fn root() -> PathBuf {
    crate::paths::documents_dir()
}

pub fn doc_dir(root: &Path, id: &str) -> AppResult<PathBuf> {
    if !is_safe_name(id) {
        return Err(AppError::Internal(format!("bad document id {id:?}")));
    }
    Ok(root.join(id))
}

/// Temp file + rename, so a crash never leaves half a file.
fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(tmp, path)
}

pub fn write_meta(dir: &Path, meta: &DocMeta) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(meta).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(write_atomic(&dir.join(META), &bytes)?)
}

pub fn read_meta(dir: &Path) -> Option<DocMeta> {
    let text = fs::read_to_string(dir.join(META)).ok()?;
    let meta: DocMeta = serde_json::from_str(&text).ok()?;
    is_safe_name(&meta.id).then_some(meta)
}

pub fn write_chunks(dir: &Path, chunks: &[Chunk]) -> AppResult<()> {
    let bytes = serde_json::to_vec(chunks).map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(write_atomic(&dir.join(CHUNKS), &bytes)?)
}

pub fn read_chunks(dir: &Path) -> Option<Vec<Chunk>> {
    serde_json::from_slice(&fs::read(dir.join(CHUNKS)).ok()?).ok()
}

pub fn write_vectors(dir: &Path, vectors: &[Vec<f32>]) -> AppResult<()> {
    let bytes: Vec<u8> = vectors.iter().flatten().flat_map(|x| x.to_le_bytes()).collect();
    Ok(write_atomic(&dir.join(VECTORS), &bytes)?)
}

pub fn read_vectors(dir: &Path) -> Option<Vec<f32>> {
    let bytes = fs::read(dir.join(VECTORS)).ok()?;
    if bytes.len() % 4 != 0 {
        return None;
    }
    Some(bytes.chunks_exact(4).map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]])).collect())
}

/// All documents, newest first.
pub fn list(root: &Path) -> Vec<DocMeta> {
    let mut docs: Vec<DocMeta> = fs::read_dir(root)
        .map(|entries| entries.flatten().filter_map(|e| read_meta(&e.path())).collect())
        .unwrap_or_default();
    docs.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| a.name.cmp(&b.name)));
    docs
}

pub fn remove(root: &Path, id: &str) -> AppResult<()> {
    match fs::remove_dir_all(doc_dir(root, id)?) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(AppError::Disk(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kuttylm-docs-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn meta(id: &str, created_at: u64) -> DocMeta {
        DocMeta {
            id: id.into(),
            name: format!("{id}.pdf"),
            kind: DocKind::Pdf,
            size_bytes: 10,
            pages: Some(1),
            chunks: 1,
            created_at,
            status: DocStatus::Ready,
            error: None,
            model_id: None,
            dim: 2,
        }
    }

    #[test]
    fn save_list_and_remove() {
        let root = temp_root("list");
        for (id, t) in [("a", 1), ("b", 2)] {
            let dir = doc_dir(&root, id).unwrap();
            fs::create_dir_all(&dir).unwrap();
            write_meta(&dir, &meta(id, t)).unwrap();
        }
        fs::create_dir_all(root.join("junk")).unwrap();
        let ids: Vec<String> = list(&root).into_iter().map(|d| d.id).collect();
        assert_eq!(ids, vec!["b", "a"]);
        remove(&root, "a").unwrap();
        remove(&root, "a").unwrap();
        assert_eq!(list(&root).len(), 1);
        assert!(doc_dir(&root, "../x").is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn vectors_round_trip() {
        let root = temp_root("vec");
        write_vectors(&root, &[vec![0.5, -1.0], vec![2.0, 0.25]]).unwrap();
        assert_eq!(read_vectors(&root).unwrap(), vec![0.5, -1.0, 2.0, 0.25]);
        let chunks = vec![Chunk { page: Some(3), text: "hello".into() }];
        write_chunks(&root, &chunks).unwrap();
        assert_eq!(read_chunks(&root).unwrap(), chunks);
        fs::remove_dir_all(root).unwrap();
    }
}
