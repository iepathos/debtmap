//! Pure editor argument construction, independent of processes and terminals.

use std::ffi::OsString;
use std::path::Path;

const PLUS_LINE_EDITORS: &[&str] = &["vim", "nvim", "vi", "emacs", "nano"];
const GOTO_EDITORS: &[&str] = &["code", "code-insiders"];
const COLON_LINE_EDITORS: &[&str] = &["subl", "sublime", "sublime_text", "hx", "helix"];

pub(super) fn arguments(editor: &str, path: &Path, line: Option<usize>) -> Vec<OsString> {
    let Some(line) = line else {
        return vec![path.into()];
    };
    match editor {
        name if PLUS_LINE_EDITORS.contains(&name) => {
            vec![format!("+{line}").into(), path.into()]
        }
        name if GOTO_EDITORS.contains(&name) => {
            vec!["--goto".into(), format!("{}:{line}", path.display()).into()]
        }
        name if COLON_LINE_EDITORS.contains(&name) => {
            vec![format!("{}:{line}", path.display()).into()]
        }
        _ => vec![path.into()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EDITORS: &[(&str, &[&str])] = &[
        ("vim", &["+42", "my project/test.rs"]),
        ("nvim", &["+42", "my project/test.rs"]),
        ("vi", &["+42", "my project/test.rs"]),
        ("emacs", &["+42", "my project/test.rs"]),
        ("nano", &["+42", "my project/test.rs"]),
        ("code", &["--goto", "my project/test.rs:42"]),
        ("code-insiders", &["--goto", "my project/test.rs:42"]),
        ("subl", &["my project/test.rs:42"]),
        ("sublime", &["my project/test.rs:42"]),
        ("sublime_text", &["my project/test.rs:42"]),
        ("hx", &["my project/test.rs:42"]),
        ("helix", &["my project/test.rs:42"]),
    ];

    #[test]
    fn supported_editors_receive_line_arguments() {
        let path = Path::new("my project/test.rs");
        for (editor, expected) in EDITORS {
            assert_eq!(arguments(editor, path, Some(42)), *expected, "{editor}");
        }
    }

    #[test]
    fn missing_line_passes_only_the_path() {
        let path = Path::new("my project/test.rs");
        for (editor, _) in EDITORS {
            assert_eq!(arguments(editor, path, None), [path.as_os_str()]);
        }
    }

    #[test]
    fn unrecognized_executables_pass_only_the_path() {
        let path = Path::new("my project/test.rs");
        for editor in ["custom", "/usr/bin/vim", "vim -f", ""] {
            for line in [None, Some(42)] {
                assert_eq!(arguments(editor, path, line), [path.as_os_str()]);
            }
        }
    }

    #[test]
    fn zero_line_is_preserved() {
        let path = Path::new("test.rs");
        assert_eq!(arguments("vim", path, Some(0)), ["+0", "test.rs"]);
        assert_eq!(arguments("code", path, Some(0)), ["--goto", "test.rs:0"]);
        assert_eq!(arguments("hx", path, Some(0)), ["test.rs:0"]);
    }

    #[cfg(unix)]
    #[test]
    fn plain_path_arguments_preserve_non_utf8_bytes() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(OsStr::from_bytes(b"test\xff.rs"));
        assert_eq!(arguments("custom", path, Some(42)), [path.as_os_str()]);
        assert_eq!(arguments("code", path, None), [path.as_os_str()]);
        assert_eq!(arguments("vim", path, Some(42))[1], path.as_os_str());
    }
}
