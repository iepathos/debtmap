use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Language> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "rs" => Some(Language::Rust),
                "py" => Some(Language::Python),
                _ => None,
            })
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_detection() {
        let path = Path::new("src/main.rs");
        assert_eq!(Language::from_path(path), Some(Language::Rust));
    }

    #[test]
    fn test_python_detection() {
        let path = Path::new("src/main.py");
        assert_eq!(Language::from_path(path), Some(Language::Python));
    }

    #[test]
    fn test_unsupported_extension() {
        let path = Path::new("README.md");
        assert_eq!(Language::from_path(path), None);
    }

    #[test]
    fn test_file_extension() {
        assert_eq!(Language::Rust.file_extension(), "rs");
        assert_eq!(Language::Python.file_extension(), "py");
    }

    #[test]
    fn test_display_name() {
        assert_eq!(Language::Rust.display_name(), "Rust");
        assert_eq!(Language::Python.display_name(), "Python");
    }
}
