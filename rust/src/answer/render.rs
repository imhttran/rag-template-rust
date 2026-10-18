//! Page reference rendering.
//!
//! Ported from the Go baseline render step. Renders a passage's page
//! reference as `source, page N` when a page is present, else just `source`.

use super::context_budget::ContextPassage;

/// Renders the page reference for `passage`.
pub fn render_page(passage: &ContextPassage) -> String {
    match passage.page {
        Some(page) => format!("{}, page {}", passage.source, page),
        None => passage.source.clone(),
    }
}

/// Renders the page reference for each passage, in order.
pub fn render_pages(passages: &[ContextPassage]) -> Vec<String> {
    passages.iter().map(render_page).collect()
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
    fn renders_source_and_page() {
        assert_eq!(render_page(&passage("doc.md", Some(4))), "doc.md, page 4");
    }

    #[test]
    fn renders_source_only_when_no_page() {
        assert_eq!(render_page(&passage("doc.md", None)), "doc.md");
    }

    #[test]
    fn renders_each_in_order() {
        let out = render_pages(&[passage("a.md", Some(1)), passage("b.md", None)]);
        assert_eq!(out, vec!["a.md, page 1".to_string(), "b.md".to_string()]);
    }
}
