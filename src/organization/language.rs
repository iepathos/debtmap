use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
}

impl Language {
    pub fn from_path(path: &Path) -> Option<Language> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| match ext {
                "rs" => Some(Language::Rust),
                "py" => Some(Language::Python),
                "js" | "jsx" => Some(Language::JavaScript),
                "ts" | "tsx" => Some(Language::TypeScript),
                _ => None,
            })
    }

    pub fn file_extension(&self) -> &'static str {
        match self {
            Language::Rust => "rs",
            Language::Python => "py",
            Language::JavaScript => "js",
            Language::TypeScript => "ts",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Language::Rust => "Rust",
            Language::Python => "Python",
            Language::JavaScript => "JavaScript",
            Language::TypeScript => "TypeScript",
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
    fn test_javascript_detection() {
        assert_eq!(
            Language::from_path(Path::new("src/app.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            Language::from_path(Path::new("src/app.jsx")),
            Some(Language::JavaScript)
        );
    }

    #[test]
    fn test_typescript_detection() {
        assert_eq!(
            Language::from_path(Path::new("src/app.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            Language::from_path(Path::new("src/app.tsx")),
            Some(Language::TypeScript)
        );
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
        assert_eq!(Language::JavaScript.file_extension(), "js");
        assert_eq!(Language::TypeScript.file_extension(), "ts");
    }

    #[test]
    fn test_display_name() {
        assert_eq!(Language::Rust.display_name(), "Rust");
        assert_eq!(Language::Python.display_name(), "Python");
        assert_eq!(Language::JavaScript.display_name(), "JavaScript");
        assert_eq!(Language::TypeScript.display_name(), "TypeScript");
    }
}
