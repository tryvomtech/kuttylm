//! Saved chats: one JSON file per chat in `<data>/kuttylm/chats/<id>.json`.
//! Chats stay on this computer only.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::catalog::types::is_safe_name;
use crate::error::{AppError, AppResult};
use crate::paths;

/// A chat file bigger than this is refused (a normal chat is a few KB).
const MAX_CHAT_BYTES: usize = 5 * 1024 * 1024;
const MAX_TITLE_CHARS: usize = 120;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredMessage {
    /// "user", "assistant" or "notice".
    pub role: String,
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_per_second: Option<f64>,
    #[serde(default)]
    pub stopped: bool,
    #[serde(default)]
    pub cut: bool,
    /// Document parts the answer was based on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    pub doc_id: String,
    pub doc_name: String,
    #[serde(default)]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chat {
    pub id: String,
    pub title: String,
    /// "question" (first words of the question), "ai" (written by the model) or "user" (renamed).
    #[serde(default)]
    pub title_by: String,
    /// Unix seconds.
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub model_name: Option<String>,
    #[serde(default)]
    pub messages: Vec<StoredMessage>,
    /// Documents chosen for this chat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doc_ids: Vec<String>,
}

/// What the history list needs; the messages are skipped while reading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatSummary {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    #[serde(default)]
    pub model_name: Option<String>,
}

fn chat_file(dir: &Path, id: &str) -> AppResult<PathBuf> {
    if !is_safe_name(id) {
        return Err(AppError::Internal(format!("bad chat id {id:?}")));
    }
    Ok(dir.join(format!("{id}.json")))
}

fn list_in(dir: &Path) -> Vec<ChatSummary> {
    let mut chats: Vec<ChatSummary> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .filter_map(|e| fs::read_to_string(e.path()).ok())
                .filter_map(|text| serde_json::from_str::<ChatSummary>(&text).ok())
                .filter(|c| is_safe_name(&c.id))
                .collect()
        })
        .unwrap_or_default();
    // Newest first.
    chats.sort_by(|a, b| b.updated_at.cmp(&a.updated_at).then_with(|| b.created_at.cmp(&a.created_at)));
    chats
}

fn load_in(dir: &Path, id: &str) -> AppResult<Chat> {
    let text = fs::read_to_string(chat_file(dir, id)?)
        .map_err(|_| AppError::Internal("This chat was not found. It may have been deleted.".into()))?;
    serde_json::from_str(&text).map_err(|e| AppError::Internal(format!("this chat file is damaged: {e}")))
}

fn save_in(dir: &Path, mut chat: Chat) -> AppResult<()> {
    let path = chat_file(dir, &chat.id)?;
    chat.title = chat.title.trim().chars().take(MAX_TITLE_CHARS).collect();
    if chat.title.is_empty() {
        chat.title = "New chat".into();
    }
    let data = serde_json::to_vec_pretty(&chat).map_err(|e| AppError::Internal(e.to_string()))?;
    if data.len() > MAX_CHAT_BYTES {
        return Err(AppError::Internal("This chat is too long to save. Start a new chat.".into()));
    }
    fs::create_dir_all(dir)?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, data)?;
    fs::rename(tmp, path)?;
    Ok(())
}

fn delete_in(dir: &Path, id: &str) -> AppResult<()> {
    match fs::remove_file(chat_file(dir, id)?) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

fn delete_all_in(dir: &Path) -> AppResult<()> {
    for chat in list_in(dir) {
        delete_in(dir, &chat.id)?;
    }
    Ok(())
}

// ---------- commands ----------

#[tauri::command]
pub async fn list_chats() -> Vec<ChatSummary> {
    tauri::async_runtime::spawn_blocking(|| list_in(&paths::chats_dir())).await.unwrap_or_default()
}

#[tauri::command]
pub fn get_chat(id: String) -> AppResult<Chat> {
    load_in(&paths::chats_dir(), &id)
}

#[tauri::command]
pub fn save_chat(chat: Chat) -> AppResult<()> {
    save_in(&paths::chats_dir(), chat)
}

#[tauri::command]
pub fn delete_chat(id: String) -> AppResult<()> {
    delete_in(&paths::chats_dir(), &id)
}

#[tauri::command]
pub fn delete_all_chats() -> AppResult<()> {
    delete_all_in(&paths::chats_dir())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kuttylm-chats-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn chat(id: &str, updated_at: u64) -> Chat {
        Chat {
            id: id.into(),
            title: format!("Chat {id}"),
            title_by: "question".into(),
            created_at: 1,
            updated_at,
            model_id: Some("m".into()),
            model_name: Some("Model".into()),
            messages: vec![
                StoredMessage {
                    role: "user".into(),
                    content: "Hello".into(),
                    reasoning: None,
                    model_name: None,
                    tokens_per_second: None,
                    stopped: false,
                    cut: false,
                    sources: vec![],
                },
                StoredMessage {
                    role: "assistant".into(),
                    content: "Hi! **How** can I help?".into(),
                    reasoning: Some("thinking".into()),
                    model_name: Some("Model".into()),
                    tokens_per_second: Some(20.5),
                    stopped: false,
                    cut: false,
                    sources: vec![Source { doc_id: "d1".into(), doc_name: "a.pdf".into(), page: Some(2) }],
                },
            ],
            doc_ids: vec!["d1".into()],
        }
    }

    #[test]
    fn save_load_list_delete() {
        let dir = temp_dir("roundtrip");
        save_in(&dir, chat("aaa", 10)).unwrap();
        save_in(&dir, chat("bbb", 30)).unwrap();
        save_in(&dir, chat("ccc", 20)).unwrap();

        assert_eq!(load_in(&dir, "aaa").unwrap(), chat("aaa", 10));
        let ids: Vec<_> = list_in(&dir).into_iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["bbb", "ccc", "aaa"], "newest first");

        delete_in(&dir, "ccc").unwrap();
        delete_in(&dir, "ccc").unwrap(); // deleting twice is fine
        assert_eq!(list_in(&dir).len(), 2);

        delete_all_in(&dir).unwrap();
        assert!(list_in(&dir).is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn unsafe_ids_are_refused() {
        let dir = temp_dir("unsafe");
        assert!(save_in(&dir, chat("../evil", 1)).is_err());
        assert!(load_in(&dir, "../../etc/passwd").is_err());
        assert!(delete_in(&dir, "a/b").is_err());
    }

    #[test]
    fn title_is_cleaned() {
        let dir = temp_dir("title");
        let mut c = chat("t1", 1);
        c.title = format!("  {}  ", "x".repeat(300));
        save_in(&dir, c).unwrap();
        assert_eq!(load_in(&dir, "t1").unwrap().title.len(), MAX_TITLE_CHARS);

        let mut c = chat("t2", 1);
        c.title = "   ".into();
        save_in(&dir, c).unwrap();
        assert_eq!(load_in(&dir, "t2").unwrap().title, "New chat");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn broken_files_are_skipped_in_the_list() {
        let dir = temp_dir("broken");
        save_in(&dir, chat("good", 1)).unwrap();
        fs::write(dir.join("bad.json"), "{ not json").unwrap();
        fs::write(dir.join("notes.txt"), "ignore me").unwrap();
        let ids: Vec<_> = list_in(&dir).into_iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["good"]);
        let _ = fs::remove_dir_all(dir);
    }
}
