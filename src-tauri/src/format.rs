/// 4370000000 → "4.1 GB" (1 GB = 1024³ bytes, same as the UI).
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn human_bytes() {
        assert_eq!(super::human_bytes(500), "500 B");
        assert_eq!(super::human_bytes(1536), "1.5 KB");
        assert_eq!(super::human_bytes(4_370_000_000), "4.1 GB");
    }
}
