//! Reads the text of a document: PDF (per page), Word (.docx), text and Markdown.

use std::io::{Cursor, Read};

use quick_xml::events::Event;
use quick_xml::Reader;

/// Part of a document's text. PDFs have one section per page.
#[derive(Debug, Clone, PartialEq)]
pub struct Section {
    /// 1-based page number (PDF only).
    pub page: Option<u32>,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DocKind {
    Pdf,
    Docx,
    Text,
    Markdown,
}

impl DocKind {
    pub fn from_name(name: &str) -> Option<Self> {
        let lower = name.to_lowercase();
        let ext = lower.rsplit_once('.').map(|(_, e)| e)?;
        Some(match ext {
            "pdf" => Self::Pdf,
            "docx" => Self::Docx,
            "txt" | "text" => Self::Text,
            "md" | "markdown" => Self::Markdown,
            _ => return None,
        })
    }
}

pub fn extract(kind: DocKind, bytes: &[u8]) -> Result<Vec<Section>, String> {
    let sections = match kind {
        DocKind::Text | DocKind::Markdown => vec![Section { page: None, text: String::from_utf8_lossy(bytes).into_owned() }],
        DocKind::Docx => vec![Section { page: None, text: docx_text(bytes)? }],
        DocKind::Pdf => pdf_pages(bytes)?,
    };
    let sections: Vec<Section> = sections
        .into_iter()
        .map(|s| Section { text: clean(&s.text), ..s })
        .filter(|s| !s.text.trim().is_empty())
        .collect();
    if sections.is_empty() {
        return Err(match kind {
            DocKind::Pdf => "No text was found. It may be a scanned PDF (pictures of pages), which is not supported yet.".into(),
            _ => "The document has no text.".into(),
        });
    }
    Ok(sections)
}

/// Normalises line ends, removes control characters and long runs of blank lines.
fn clean(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(text.len());
    let mut blank = 0;
    for line in text.lines() {
        let line: String = line.chars().filter(|c| !c.is_control() || *c == '\t').collect();
        let line = line.trim_end();
        if line.trim().is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim().to_string()
}

fn pdf_pages(bytes: &[u8]) -> Result<Vec<Section>, String> {
    // The PDF library can panic on unusual files. Release builds use panic = "unwind",
    // so the panic is caught here instead of closing the app.
    let owned = bytes.to_vec();
    let result = std::panic::catch_unwind(move || pdf_extract::extract_text_from_mem_by_pages(&owned));
    match result {
        Ok(Ok(pages)) => Ok(pages
            .into_iter()
            .enumerate()
            .map(|(i, text)| Section { page: Some(i as u32 + 1), text })
            .collect()),
        Ok(Err(e)) => Err(format!("This PDF could not be read ({e}). It may be protected with a password or damaged.")),
        Err(_) => Err("This PDF could not be read. It may be damaged or use an unusual format.".into()),
    }
}

/// The text of `word/document.xml`: paragraphs become lines, tabs and breaks are kept.
fn docx_text(bytes: &[u8]) -> Result<String, String> {
    let bad = || "This Word file could not be read. Is it a .docx file (not the old .doc)?".to_string();
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).map_err(|_| bad())?;
    let mut xml = String::new();
    {
        let mut file = archive.by_name("word/document.xml").map_err(|_| bad())?;
        // A normal document.xml is a few MB at most; refuse "zip bombs".
        if file.size() > 100 * 1024 * 1024 {
            return Err(bad());
        }
        file.read_to_string(&mut xml).map_err(|_| bad())?;
    }

    let mut reader = Reader::from_str(&xml);
    let mut out = String::new();
    let mut in_text = false;
    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) if e.name().into_inner() == "w:t" => in_text = true,
            Ok(Event::End(e)) => match e.name().into_inner() {
                "w:t" => in_text = false,
                "w:p" => out.push('\n'),
                _ => {}
            },
            Ok(Event::Empty(e)) => match e.name().into_inner() {
                "w:tab" => out.push('\t'),
                "w:br" | "w:cr" => out.push('\n'),
                _ => {}
            },
            Ok(Event::Text(t)) if in_text => {
                let raw = AsRef::<str>::as_ref(&t).to_string();
                let text = quick_xml::escape::unescape(&raw).map(|c| c.into_owned()).unwrap_or(raw);
                out.push_str(&text);
            }
            Ok(Event::GeneralRef(r)) if in_text => {
                // &amp; &lt; … inside text (quick-xml 0.38+ reports them separately).
                let name = AsRef::<str>::as_ref(&r).to_string();
                let text = quick_xml::escape::unescape(&format!("&{name};")).map(|c| c.into_owned()).unwrap_or_default();
                out.push_str(&text);
            }
            Ok(Event::Eof) => break,
            Err(_) => return Err(bad()),
            _ => {}
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn kinds() {
        assert_eq!(DocKind::from_name("Report.PDF"), Some(DocKind::Pdf));
        assert_eq!(DocKind::from_name("notes.md"), Some(DocKind::Markdown));
        assert_eq!(DocKind::from_name("a.docx"), Some(DocKind::Docx));
        assert_eq!(DocKind::from_name("old.doc"), None);
        assert_eq!(DocKind::from_name("noext"), None);
    }

    #[test]
    fn text_is_cleaned() {
        let s = extract(DocKind::Text, b"Line one\r\n\r\n\r\n\r\nLine two\x07\n").unwrap();
        assert_eq!(s, vec![Section { page: None, text: "Line one\n\nLine two".into() }]);
        assert!(extract(DocKind::Text, b"   \n  ").is_err());
    }

    /// Builds a tiny .docx in memory.
    fn docx(body: &str) -> Vec<u8> {
        let mut buf = Cursor::new(Vec::new());
        {
            let mut zip = zip::ZipWriter::new(&mut buf);
            zip.start_file("word/document.xml", zip::write::SimpleFileOptions::default()).unwrap();
            write!(zip, r#"<?xml version="1.0"?><w:document xmlns:w="w"><w:body>{body}</w:body></w:document>"#).unwrap();
            zip.finish().unwrap();
        }
        buf.into_inner()
    }

    #[test]
    fn word_documents() {
        let bytes = docx(
            r#"<w:p><w:r><w:t>Hello</w:t></w:r><w:r><w:tab/><w:t xml:space="preserve"> Tom &amp; Jerry</w:t></w:r></w:p><w:p><w:r><w:t>Second</w:t><w:br/><w:t>line</w:t></w:r></w:p>"#,
        );
        let s = extract(DocKind::Docx, &bytes).unwrap();
        assert_eq!(s[0].text, "Hello\t Tom & Jerry\nSecond\nline");
        assert!(extract(DocKind::Docx, b"not a zip").is_err());
    }

    #[test]
    fn broken_pdf_gives_a_message_not_a_crash() {
        let err = extract(DocKind::Pdf, b"%PDF-1.4 this is not really a pdf").unwrap_err();
        assert!(err.contains("PDF"), "{err}");
    }
}
