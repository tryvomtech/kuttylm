use std::time::{SystemTime, UNIX_EPOCH};

/// Random bytes from the system, as hex text (for keys).
pub fn random_hex(bytes: usize) -> std::io::Result<String> {
    let mut buf = vec![0u8; bytes];
    getrandom::fill(&mut buf).map_err(|e| std::io::Error::other(e.to_string()))?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

/// Current Unix time in seconds.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    #[test]
    fn random_hex_is_random() {
        let a = super::random_hex(16).unwrap();
        assert_eq!(a.len(), 32);
        assert_ne!(a, super::random_hex(16).unwrap());
    }
}
