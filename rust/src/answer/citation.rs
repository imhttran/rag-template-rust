//! Citation parsing, validation and repair.
//!
//! Ported from the Go baseline citation rules. Citations use the form
//! `[page N]` or `[source, page N]`. Parsing extracts citations from a model
//! answer; validation checks them against the retrieved evidence; repair drops
//! citations that reference evidence not present in the context.

use super::context_budget::ContextPassage;

/// A parsed citation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Citation {
    /// Optional source document name.
    pub source: Option<String>,
    /// Optional 1-based page number.
    pub page: Option<u32>,
    /// The raw matched text, e.g. `[page 3]`.
    pub raw: String,
}

/// Parses all citations from `answer` in order of appearance.
///
/// Recognises `[page N]`, `[source, page N]`, and `[source]` forms.
pub fn parse(answer: &str) -> Vec<Citation> {
    let mut citations = Vec::new();
    let mut rest = answer;
    while let Some(open) = rest.find('[') {
        let after_open = &rest[open..];
        let Some(close) = after_open.find(']') else {
            break;
        };
        let raw = &after_open[..=close];
        if let Some(citation) = parse_one(raw) {
            citations.push(citation);
        }
        rest = &after_open[close + 1..];
    }
    citations
}

fn parse_one(raw: &str) -> Option<Citation> {
    let inner = raw.strip_prefix('[')?.strip_suffix(']')?.trim();
    let lower = inner.to_lowercase();
    if let Some(rest) = lower.strip_prefix("page ") {
        let page = rest.trim().parse::<u32>().ok()?;
        return Some(Citation {
            source: None,
            page: Some(page),
            raw: raw.to_string(),
        });
    }
    if let Some((source, rest)) = inner.split_once(',') {
        let rest_lower = rest.trim().to_lowercase();
        if let Some(page_str) = rest_lower.strip_prefix("page ") {
            let page = page_str.trim().parse::<u32>().ok()?;
            return Some(Citation {
                source: Some(source.trim().to_string()),
                page: Some(page),
                raw: raw.to_string(),
            });
        }
    }
    if !inner.is_empty() && inner.split_whitespace().count() == 1 {
        return Some(Citation {
            source: Some(inner.to_string()),
            page: None,
            raw: raw.to_string(),
        });
    }
    None
}

/// Returns `true` when `citation` matches some passage in `passages`.
pub fn is_valid(citation: &Citation, passages: &[ContextPassage]) -> bool {
    passages.iter().any(|p| {
        let source_ok = citation
            .source
            .as_ref()
            .map(|s| s == &p.source)
            .unwrap_or(true);
        let page_ok = citation.page.map(|pg| Some(pg) == p.page).unwrap_or(true);
        source_ok && page_ok
    })
}

/// Repairs `answer` by dropping invalid citations.
///
/// Each bracketed citation that does not validate against `passages` is
/// removed from the text. Valid citations are kept verbatim; non-citation text
/// is preserved.
pub fn repair(answer: &str, passages: &[ContextPassage]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < answer.len() {
        if answer.as_bytes()[i] == b'[' {
            if let Some(close) = answer[i..].find(']') {
                let raw = &answer[i..=i + close];
                if let Some(citation) = parse_one(raw) {
                    if is_valid(&citation, passages) {
                        out.push_str(raw);
                    }
                    i += close + 1;
                    continue;
                }
            }
        }
        let ch = answer[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passage(source: &str, page: Option<u32>) -> ContextPassage {
        ContextPassage {
            source: source.to_string(),
            section: "S".to_string(),
            page,
            content: "body".to_string(),
        }
    }

    #[test]
    fn parses_page_citation() {
        let c = parse("see [page 3].");
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].page, Some(3));
        assert_eq!(c[0].source, None);
    }

    #[test]
    fn parses_source_and_page_citation() {
        let c = parse("[doc.md, page 2]");
        assert_eq!(c[0].source.as_deref(), Some("doc.md"));
        assert_eq!(c[0].page, Some(2));
    }

    #[test]
    fn validates_against_passages() {
        let passages = vec![passage("doc.md", Some(2))];
        let valid = parse("[doc.md, page 2]");
        let invalid = parse("[doc.md, page 9]");
        assert!(is_valid(&valid[0], &passages));
        assert!(!is_valid(&invalid[0], &passages));
    }

    #[test]
    fn repair_drops_invalid_citations() {
        let passages = vec![passage("doc.md", Some(2))];
        let answer = "Fact [doc.md, page 2] and bogus [doc.md, page 9].";
        let repaired = repair(answer, &passages);
        assert!(repaired.contains("[doc.md, page 2]"));
        assert!(!repaired.contains("[doc.md, page 9]"));
    }

    #[test]
    fn plain_text_without_citations_is_unchanged_by_parse() {
        assert!(parse("no citations here").is_empty());
    }
}
