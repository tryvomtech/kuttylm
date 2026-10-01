//! Splits document text into overlapping parts ("chunks") for search.

use serde::{Deserialize, Serialize};

use super::parse::Section;

/// Aim for parts of about this many characters (roughly 250 tokens).
pub const TARGET_CHARS: usize = 1000;
/// Each part repeats the end of the previous one, so no sentence is lost at a border.
pub const OVERLAP_CHARS: usize = 150;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chunk {
    pub page: Option<u32>,
    pub text: String,
}

/// The best place to cut `text` before `max` bytes: paragraph, line, sentence, then word end.
fn cut_point(text: &str, max: usize) -> usize {
    if text.len() <= max {
        return text.len();
    }
    let mut limit = max;
    while !text.is_char_boundary(limit) {
        limit -= 1;
    }
    let window = &text[..limit];
    let min = limit / 2; // never cut tiny parts
    for sep in ["\n\n", "\n", ". ", "? ", "! ", "; ", ", ", " "] {
        if let Some(i) = window.rfind(sep) {
            if i >= min {
                return i + sep.len();
            }
        }
    }
    limit
}

/// Start of the overlap: a word start about `OVERLAP_CHARS` before `end`.
fn overlap_start(text: &str, end: usize) -> usize {
    let mut start = end.saturating_sub(OVERLAP_CHARS);
    while !text.is_char_boundary(start) {
        start += 1;
    }
    match text[start..end].find(char::is_whitespace) {
        Some(i) => start + i + 1,
        None => end,
    }
}

pub fn chunk_sections(sections: &[Section]) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    for section in sections {
        let text = section.text.trim();
        let mut start = 0;
        while start < text.len() {
            let end = start + cut_point(&text[start..], TARGET_CHARS);
            let piece = text[start..end].trim();
            if !piece.is_empty() {
                chunks.push(Chunk { page: section.page, text: piece.to_string() });
            }
            if end >= text.len() {
                break;
            }
            let next = overlap_start(text, end);
            // Always move forward.
            start = if next > start { next } else { end };
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn section(text: &str, page: Option<u32>) -> Section {
        Section { page, text: text.into() }
    }

    #[test]
    fn short_text_is_one_chunk() {
        let c = chunk_sections(&[section("Just one short paragraph.", Some(2))]);
        assert_eq!(c, vec![Chunk { page: Some(2), text: "Just one short paragraph.".into() }]);
    }

    #[test]
    fn long_text_is_split_with_overlap() {
        let sentence = "The quick brown fox jumps over the lazy dog. ";
        let text = sentence.repeat(80); // about 3,600 characters
        let c = chunk_sections(&[section(&text, None)]);
        assert!(c.len() >= 4 && c.len() <= 6, "{} chunks", c.len());
        for part in &c {
            assert!(part.text.len() <= TARGET_CHARS + 5, "too long: {}", part.text.len());
            assert!(part.text.starts_with("The") || part.text.starts_with("quick") || part.text.chars().next().unwrap().is_alphabetic());
        }
        // Cuts happen at sentence ends.
        assert!(c[0].text.ends_with('.'));
        // Consecutive parts share some words.
        let tail: String = c[0].text.chars().rev().take(40).collect::<String>().chars().rev().collect();
        assert!(c[1].text.contains(tail.trim().split(' ').last().unwrap()));
    }

    #[test]
    fn pages_are_kept() {
        let c = chunk_sections(&[section("Page one text.", Some(1)), section("Page two text.", Some(2))]);
        assert_eq!(c.iter().map(|x| x.page).collect::<Vec<_>>(), vec![Some(1), Some(2)]);
    }

    #[test]
    fn no_endless_loop_on_text_without_spaces() {
        let text = "x".repeat(5000);
        let c = chunk_sections(&[section(&text, None)]);
        assert_eq!(c.len(), 5);
    }

    #[test]
    fn non_ascii_text_is_cut_safely() {
        let text = "தமிழ் உரை சோதனை. ".repeat(200);
        let c = chunk_sections(&[section(&text, None)]);
        assert!(c.len() > 1);
    }
}
