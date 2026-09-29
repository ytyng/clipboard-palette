//! Open mode: while Shift is held, a card whose text is a URL or a file path
//! opens it instead of copying it.
//!
//! The frontend only sends the index of the clicked item. The text to open is
//! read back from the app state and classified again here, so the web view can
//! never ask the app to open an arbitrary string.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::{Manager, State};

use crate::AppState;

/// What a card opens in open mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenKind {
    /// An http(s) URL, opened in the default browser
    Url,
    /// A file path, revealed in Finder (the file manager elsewhere)
    Path,
}

/// Decide whether the text is something open mode can open.
///
/// Only a single line counts, with the surrounding whitespace ignored. URLs are
/// limited to http and https: other schemes (file:, javascript:, custom app
/// schemes) could start programs rather than show a page.
pub fn classify(text: &str) -> Option<OpenKind> {
    let text = text.trim();
    if text.is_empty() || text.chars().any(char::is_control) {
        return None;
    }
    if is_web_url(text) {
        return Some(OpenKind::Url);
    }
    if looks_like_path(text) {
        return Some(OpenKind::Path);
    }
    None
}

fn is_web_url(text: &str) -> bool {
    // A URL never holds whitespace
    if text.contains(char::is_whitespace) {
        return false;
    }
    let lower = text.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    else {
        return false;
    };
    // The host has to be written right after "//". The parser is lenient here
    // ("https:///path" becomes host "path"), so check the text itself
    let authority = rest.split(['/', '\\', '?', '#']).next().unwrap_or("");
    if authority.is_empty() {
        return false;
    }
    tauri::Url::parse(text)
        .map(|url| url.host_str().is_some_and(|host| !host.is_empty()))
        .unwrap_or(false)
}

fn looks_like_path(text: &str) -> bool {
    // Unix style: absolute, home relative, or explicitly relative
    if text.starts_with('/')
        || text == "~"
        || text.starts_with("~/")
        || text.starts_with("./")
        || text.starts_with("../")
    {
        return true;
    }
    // Windows style, only on Windows: elsewhere a drive letter or UNC path is
    // just a relative path and would open some unrelated folder
    cfg!(windows) && is_windows_path(text)
}

/// Drive letter (C:\ or C:/) or a UNC path
fn is_windows_path(text: &str) -> bool {
    let bytes = text.as_bytes();
    (bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/'))
        || text.starts_with("\\\\")
}

/// Turn the text into the path to show: expand a leading ~ and resolve a
/// relative path against the directory the command was run from.
fn resolve_path(text: &str, home: Option<&Path>, cwd: Option<&Path>) -> Option<PathBuf> {
    let text = text.trim();
    let path = if text == "~" {
        home?.to_path_buf()
    } else if let Some(rest) = text.strip_prefix("~/") {
        home?.join(rest)
    } else {
        PathBuf::from(text)
    };
    if path.is_absolute() {
        Some(path)
    } else {
        Some(cwd?.join(path))
    }
}

/// Show the path in the file manager.
///
/// The path is revealed, so its folder opens with the item selected. A path
/// that does not exist (yet, or any more) reveals its nearest existing
/// ancestor instead of failing.
///
/// Always reveal, never open: opening a folder-like item such as an .app
/// bundle would launch it, and the text comes from outside the app.
fn reveal_path(path: &Path) -> Result<(), String> {
    let target = path
        .ancestors()
        .find(|p| p.exists())
        .ok_or_else(|| format!("Nothing exists along {}", path.display()))?;
    tauri_plugin_opener::reveal_item_in_dir(target).map_err(|e| e.to_string())
}

/// Open the item at the given index: a URL in the default browser, a path in
/// the file manager. Anything else is refused.
#[tauri::command]
pub fn open_item(
    app: tauri::AppHandle,
    state: State<AppState>,
    index: usize,
) -> Result<(), String> {
    let text = {
        let data = state.data.lock().unwrap();
        let item = data
            .as_ref()
            .and_then(|d| d.items.get(index))
            .ok_or_else(|| format!("No item at index {}", index))?;
        item.text.trim().to_string()
    };
    match classify(&text) {
        Some(OpenKind::Url) => {
            tauri_plugin_opener::open_url(&text, None::<&str>).map_err(|e| e.to_string())
        }
        Some(OpenKind::Path) => {
            let home = app.path().home_dir().ok();
            let cwd = std::env::current_dir().ok();
            let path = resolve_path(&text, home.as_deref(), cwd.as_deref())
                .ok_or_else(|| format!("Cannot resolve the path {}", text))?;
            reveal_path(&path)
        }
        None => Err("The item is neither a URL nor a file path".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, is_windows_path, resolve_path, OpenKind};
    use std::path::{Path, PathBuf};

    #[test]
    fn http_and_https_urls_are_opened() {
        assert_eq!(classify("https://example.com/a?b=c"), Some(OpenKind::Url));
        assert_eq!(classify("http://localhost:8080"), Some(OpenKind::Url));
        assert_eq!(classify("  HTTPS://Example.com  \n"), Some(OpenKind::Url));
    }

    #[test]
    fn other_schemes_are_not_opened() {
        // These could start programs rather than show a page
        assert_eq!(classify("file:///etc/passwd"), None);
        assert_eq!(classify("javascript:alert(1)"), None);
        assert_eq!(classify("vscode://file/tmp"), None);
        assert_eq!(classify("mailto:a@example.com"), None);
    }

    #[test]
    fn a_url_needs_a_host_and_no_spaces() {
        assert_eq!(classify("https://"), None);
        assert_eq!(classify("https:///path"), None);
        assert_eq!(classify("https://example.com and more"), None);
        assert_eq!(classify("https://?q=test"), None);
        assert_eq!(classify("https://#fragment"), None);
        assert_eq!(classify("https:example.com"), None);
    }

    #[test]
    fn paths_are_recognised() {
        for text in [
            "/Users/me/file.txt",
            "~",
            "~/Downloads",
            "./src/lib.rs",
            "../README.md",
        ] {
            assert_eq!(classify(text), Some(OpenKind::Path), "{}", text);
        }
    }

    #[test]
    fn windows_paths_count_only_on_windows() {
        let expected = if cfg!(windows) {
            Some(OpenKind::Path)
        } else {
            None
        };
        for text in ["C:\\Users\\me", "c:/tmp", "\\\\server\\share"] {
            assert!(is_windows_path(text), "{}", text);
            assert_eq!(classify(text), expected, "{}", text);
        }
    }

    #[test]
    fn plain_text_is_left_alone() {
        for text in ["", "   ", "hello", "src/lib.rs", "~user", "a/b c", "C:"] {
            assert_eq!(classify(text), None, "{:?}", text);
        }
    }

    #[test]
    fn multi_line_text_is_left_alone() {
        assert_eq!(classify("https://example.com\nhttps://example.org"), None);
        assert_eq!(classify("/tmp\n/var"), None);
    }

    // Unix paths only: on Windows "/abs" is not absolute
    #[cfg(unix)]
    #[test]
    fn home_and_relative_paths_are_resolved() {
        let home = Path::new("/home/me");
        let cwd = Path::new("/work");
        assert_eq!(
            resolve_path("~", Some(home), Some(cwd)),
            Some(PathBuf::from("/home/me"))
        );
        assert_eq!(
            resolve_path("~/a/b", Some(home), Some(cwd)),
            Some(PathBuf::from("/home/me/a/b"))
        );
        assert_eq!(
            resolve_path("./x", Some(home), Some(cwd)),
            Some(PathBuf::from("/work/./x"))
        );
        assert_eq!(
            resolve_path("/abs", None, None),
            Some(PathBuf::from("/abs"))
        );
        assert_eq!(resolve_path("~/a", None, Some(cwd)), None);
    }
}
