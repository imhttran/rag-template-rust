//! Clean-checkout acceptance harness (Go present).
//!
//! Copies the working tree into a temporary directory (excluding `.git`, build
//! outputs, and local state), reads the documented setup sequence from
//! `docs/guides/usage.md`, and runs the documented Go and Rust commands there.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Repo root, three levels above this crate's manifest directory.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("..")
}

/// The setup guide that holds the documented clean-checkout sequence.
pub fn setup_path() -> PathBuf {
    repo_root().join("docs/guides/usage.md")
}

/// The setup guide text. The harness fails when it cannot be read.
pub fn load_setup() -> Result<String, String> {
    let path = setup_path();
    std::fs::read_to_string(&path).map_err(|err| format!("read {}: {err}", path.display()))
}

/// Return the body of the `## Clean checkout with Go present` section.
pub fn setup_section<'a>(doc: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("## {heading}");
    let mut search_from = 0;
    while let Some(offset) = doc[search_from..].find(&marker) {
        let start = search_from + offset;
        let at_line_start = start == 0 || doc.as_bytes()[start - 1] == b'\n';
        if at_line_start {
            let rest = &doc[start + marker.len()..];
            let end = rest.find("\n## ").map(|i| i + 1).unwrap_or(rest.len());
            return Some(&rest[..end]);
        }
        search_from = start + marker.len();
    }
    None
}

/// Every fenced ```bash block inside `section_text`, in order.
pub fn fenced_bash_blocks(section_text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Option<String> = None;
    for line in section_text.lines() {
        let fence = line.trim_start();
        match &mut current {
            None => {
                if fence.starts_with("```bash") {
                    current = Some(String::new());
                }
            }
            Some(buf) => {
                if fence.starts_with("```") {
                    blocks.push(std::mem::take(buf));
                    current = None;
                } else {
                    buf.push_str(line);
                    buf.push('\n');
                }
            }
        }
    }
    blocks
}

/// Directories never copied into the temporary checkout.
const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", "web/.run", ".agent-sdlc"];

/// Recursively copy `src` into `dst`, skipping `SKIP_DIRS` and `.env`.
pub fn copy_tree(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy().to_string();
        let rel = dst.join(&name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if SKIP_DIRS.contains(&name_str.as_str()) {
                continue;
            }
            copy_tree(&entry.path(), &rel)?;
        } else if file_type.is_symlink() {
            continue;
        } else {
            if name_str == ".env" {
                continue;
            }
            std::fs::copy(entry.path(), &rel)?;
        }
    }
    Ok(())
}

/// Copy the repository into a fresh temporary directory and return its path.
pub fn stage_clean_checkout() -> Result<PathBuf, String> {
    let root = repo_root();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dest =
        std::env::temp_dir().join(format!("rag-clean-checkout-{}-{nanos}", std::process::id()));
    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(|e| e.to_string())?;
    }
    copy_tree(&root, &dest).map_err(|e| format!("copy working tree: {e}"))?;
    Ok(dest)
}

/// Run one documented shell command inside `cwd` and return its exit status.
///
/// A clean checkout does not contain the local `.env`, so the steps inherit the
/// values from the source checkout's `.env` when the parent environment does not
/// already define them. The file is read, never copied into the staged tree.
pub fn run_step(cwd: &Path, command: &str) -> Result<std::process::ExitStatus, String> {
    let mut child = Command::new("sh");
    child.arg("-c").arg(command).current_dir(cwd);
    for (key, value) in dotenv_values(&repo_root()) {
        if std::env::var_os(&key).is_none() {
            child.env(key, value);
        }
    }
    let status = child
        .status()
        .map_err(|e| format!("spawn `{command}`: {e}"))?;
    Ok(status)
}

/// Read simple `KEY=VALUE` entries from the source checkout's `.env`.
///
/// Blank lines and `#` comments are ignored; surrounding single or double quotes
/// are removed. A missing file yields no entries.
pub fn dotenv_values(root: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(root.join(".env")) else {
        return Vec::new();
    };
    let mut values = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty() {
            continue;
        }
        let value = value.trim().trim_matches('"').trim_matches('\'');
        values.push((key.to_string(), value.to_string()));
    }
    values
}

/// The commands the harness runs from the documented setup section.
///
/// These are the acceptance commands the PRD lists; each must be documented
/// verbatim in `docs/guides/usage.md` so the harness cannot drift from the
/// documented setup.
pub const REQUIRED_SETUP_COMMANDS: &[&str] = &[
    "go build ./...",
    "go test ./...",
    "go vet ./...",
    "make rust-test",
];

#[cfg(test)]
mod tests {
    use super::dotenv_values;

    #[test]
    fn dotenv_values_skips_comments_and_strips_quotes() {
        let dir = std::env::temp_dir().join(format!(
            "dotenv-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(".env"),
            "# comment\n\nDATABASE_URL=\"postgres://example/db\"\nEXPORTED=plain\n",
        )
        .unwrap();
        let values = dotenv_values(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(values.contains(&(
            "DATABASE_URL".to_string(),
            "postgres://example/db".to_string()
        )));
        assert!(values.contains(&("EXPORTED".to_string(), "plain".to_string())));
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn missing_dotenv_yields_no_values() {
        let dir = std::env::temp_dir().join(format!(
            "dotenv-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let values = dotenv_values(&dir);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(values.is_empty());
    }
}
