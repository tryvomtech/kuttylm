//! Checks a finished file: size first (fast), then sha256 (if the list gives one).

use std::fs::File;
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

pub fn sha256_hex(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// `Ok(true)` if the file is correct. Hashing runs on a blocking thread.
pub async fn file_is_valid(path: &Path, size: u64, sha256: Option<&str>) -> io::Result<bool> {
    if std::fs::metadata(path)?.len() != size {
        return Ok(false);
    }
    let Some(expected) = sha256 else {
        return Ok(true);
    };
    let path: PathBuf = path.to_path_buf();
    let actual = tauri::async_runtime::spawn_blocking(move || sha256_hex(&path))
        .await
        .map_err(io::Error::other)??;
    Ok(actual.eq_ignore_ascii_case(expected))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_hash() {
        let path = std::env::temp_dir().join(format!("kuttylm-sha-{}", std::process::id()));
        std::fs::write(&path, b"abc").unwrap();
        assert_eq!(
            sha256_hex(&path).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_file(path);
    }
}
