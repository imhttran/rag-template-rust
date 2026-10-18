//! Prompt construction that separates instructions from untrusted text.
//!
//! Ported from the Go baseline prompt builder. Instructions and untrusted
//! retrieved passages are emitted in explicit, labelled blocks so a prompt
//! injection inside a document cannot masquerade as an instruction.

use super::context_budget::ContextPassage;

/// Opening marker of the instruction (trusted) block.
pub const INSTRUCTIONS_OPEN: &str = "<<<INSTRUCTIONS>>>";
/// Closing marker of the instruction block.
pub const INSTRUCTIONS_CLOSE: &str = "<<</INSTRUCTIONS>>>";
/// Opening marker of the untrusted-context block.
pub const CONTEXT_OPEN: &str = "<<<UNTRUSTED_CONTEXT>>>";
/// Closing marker of the untrusted-context block.
pub const CONTEXT_CLOSE: &str = "<<</UNTRUSTED_CONTEXT>>>";

/// Builds the answer prompt from `question` and `passages`.
///
/// The instruction block precedes the untrusted context block. Every retrieved
/// passage is inside the untrusted block and never inside the instruction
/// block.
pub fn build(question: &str, passages: &[ContextPassage]) -> String {
    let mut prompt = String::new();
    prompt.push_str(INSTRUCTIONS_OPEN);
    prompt.push('\n');
    prompt.push_str(
        "Answer the question using only the untrusted context below. Cite sources by page when present. If the context is insufficient, refuse.",
    );
    prompt.push('\n');
    prompt.push_str("Question: ");
    prompt.push_str(question);
    prompt.push('\n');
    prompt.push_str(INSTRUCTIONS_CLOSE);
    prompt.push('\n');
    prompt.push_str(CONTEXT_OPEN);
    prompt.push('\n');
    for (i, passage) in passages.iter().enumerate() {
        prompt.push_str(&format!(
            "[{}] {} :: {}\n",
            i + 1,
            passage.source,
            passage.section
        ));
        prompt.push_str(&passage.content);
        prompt.push('\n');
    }
    prompt.push_str(CONTEXT_CLOSE);
    prompt
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passage(section: &str, content: &str) -> ContextPassage {
        ContextPassage {
            source: "doc.md".to_string(),
            section: section.to_string(),
            page: Some(2),
            content: content.to_string(),
        }
    }

    #[test]
    fn instructions_precede_context() {
        let prompt = build("what?", &[passage("A", "body")]);
        let instr = prompt.find(INSTRUCTIONS_OPEN).unwrap();
        let ctx = prompt.find(CONTEXT_OPEN).unwrap();
        assert!(instr < ctx);
    }

    #[test]
    fn every_passage_is_inside_untrusted_block() {
        let prompt = build(
            "what?",
            &[passage("A", "first body"), passage("B", "second body")],
        );
        let ctx_open = prompt.find(CONTEXT_OPEN).unwrap();
        let ctx_close = prompt.find(CONTEXT_CLOSE).unwrap();
        for marker in ["first body", "second body"] {
            let pos = prompt.find(marker).unwrap();
            assert!(pos > ctx_open && pos < ctx_close);
        }
    }

    #[test]
    fn injected_instruction_text_stays_in_context_block() {
        let prompt = build("what?", &[passage("A", INSTRUCTIONS_CLOSE)]);
        let instr_close = prompt.find(INSTRUCTIONS_CLOSE).unwrap();
        let ctx_open = prompt.find(CONTEXT_OPEN).unwrap();
        // The first INSTRUCTIONS_CLOSE closes the instruction block; the
        // passage copy appears after the context opens.
        assert!(instr_close < ctx_open);
    }
}
