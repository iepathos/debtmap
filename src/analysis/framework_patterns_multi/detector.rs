//! Framework Pattern Detector Implementation

use super::patterns::{FrameworkMatch, FrameworkPattern, Language, PatternMatcher};
use anyhow::{Context, Result};
use dashmap::DashMap;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

/// File context for pattern matching
#[derive(Debug, Clone, Default)]
pub struct FileContext {
    /// Language of the file
    pub language: Language,
    /// Import statements
    pub imports: Vec<String>,
    /// File path
    pub path: std::path::PathBuf,
}

impl FileContext {
    /// Create new file context
    pub fn new(language: Language, path: std::path::PathBuf) -> Self {
        Self {
            language,
            imports: Vec::new(),
            path,
        }
    }

    /// Add an import statement
    pub fn add_import(&mut self, import: String) {
        self.imports.push(import);
    }
}

/// Function AST representation for pattern matching
#[derive(Debug, Clone, Default)]
pub struct FunctionAst {
    /// Function name
    pub name: String,
    /// Decorators (Python/TypeScript)
    pub decorators: Vec<Decorator>,
    /// Attributes (Rust)
    pub attributes: Vec<Attribute>,
    /// Derive macros (Rust)
    pub derives: Vec<String>,
    /// Parameters
    pub parameters: Vec<Parameter>,
    /// Return type
    pub return_type: Option<String>,
    /// Function calls in body
    pub calls: Vec<FunctionCall>,
    /// Is async function
    pub is_async: bool,
    /// Function body contains JSX
    pub body_contains_jsx: bool,
}

impl FunctionAst {
    /// Create new function AST
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }
}

/// Decorator representation
#[derive(Debug, Clone)]
pub struct Decorator {
    /// Decorator name
    pub name: String,
}

/// Attribute representation
#[derive(Debug, Clone)]
pub struct Attribute {
    /// Attribute text
    pub text: String,
}

impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.text)
    }
}

/// Parameter representation
#[derive(Debug, Clone)]
pub struct Parameter {
    /// Parameter name
    pub name: String,
    /// Type annotation
    pub type_annotation: String,
}

/// Function call representation
#[derive(Debug, Clone)]
pub struct FunctionCall {
    /// Function name
    pub name: String,
}

/// Framework pattern detector
pub struct FrameworkDetector {
    /// Patterns indexed by language
    patterns: HashMap<Language, Vec<FrameworkPattern>>,
    /// Regex cache for performance
    regex_cache: DashMap<String, Regex>,
}

impl FrameworkDetector {
    /// Create detector from TOML configuration
    pub fn from_config(config_path: &Path) -> Result<Self> {
        let config_content = std::fs::read_to_string(config_path).context(format!(
            "Failed to read config file: {}",
            config_path.display()
        ))?;

        let config: toml::Value =
            toml::from_str(&config_content).context("Failed to parse TOML configuration")?;

        let patterns = parse_config_into_patterns(&config)?;

        Ok(Self {
            patterns,
            regex_cache: DashMap::new(),
        })
    }

    /// Create detector with default embedded patterns
    pub fn with_defaults() -> Self {
        // Use embedded defaults if no config file available
        Self {
            patterns: HashMap::new(),
            regex_cache: DashMap::new(),
        }
    }

    /// Detect framework patterns in a function
    pub fn detect_framework_patterns(
        &self,
        function: &FunctionAst,
        file_context: &FileContext,
    ) -> Vec<FrameworkMatch> {
        let language = file_context.language;
        let framework_patterns = match self.patterns.get(&language) {
            Some(patterns) => patterns,
            None => return vec![],
        };

        let mut matches = Vec::new();

        for framework_pattern in framework_patterns {
            if let Some(matched) = self.match_framework(function, file_context, framework_pattern) {
                matches.push(matched);
            }
        }

        // Sort by confidence (highest first)
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        matches
    }

    /// Match a single framework pattern
    fn match_framework(
        &self,
        function: &FunctionAst,
        file_context: &FileContext,
        pattern: &FrameworkPattern,
    ) -> Option<FrameworkMatch> {
        let mut matched_count = 0;
        let total_patterns = pattern.patterns.len();
        let mut evidence = Vec::new();

        for matcher in &pattern.patterns {
            if let Some(ev) = self.matches_pattern(function, file_context, matcher) {
                matched_count += 1;
                evidence.push(ev);
            }
        }

        // Require at least one pattern match
        if matched_count == 0 {
            return None;
        }

        // Calculate confidence based on match ratio
        let confidence = if total_patterns > 0 {
            (matched_count as f64 / total_patterns as f64).max(0.5)
        } else {
            0.5
        };

        Some(
            FrameworkMatch::new(pattern.name.clone(), pattern.category.clone(), confidence)
                .with_evidence(evidence.join(", ")),
        )
    }

    /// Check if a pattern matches
    fn matches_pattern(
        &self,
        function: &FunctionAst,
        file_context: &FileContext,
        matcher: &PatternMatcher,
    ) -> Option<String> {
        match matcher {
            PatternMatcher::Import { pattern } => {
                for import in &file_context.imports {
                    if self.regex_match(pattern, import) {
                        return Some(format!("import: {}", import));
                    }
                }
                None
            }
            PatternMatcher::Decorator { pattern } => {
                for decorator in &function.decorators {
                    if self.regex_match(pattern, &decorator.name) {
                        return Some(format!("decorator: {}", decorator.name));
                    }
                }
                None
            }
            PatternMatcher::Attribute { pattern } => {
                for attr in &function.attributes {
                    let attr_str = attr.to_string();
                    if self.regex_match(pattern, &attr_str) {
                        return Some(format!("attribute: {}", attr_str));
                    }
                }
                None
            }
            PatternMatcher::Derive { pattern } => {
                for derive in &function.derives {
                    if self.regex_match(pattern, derive) {
                        return Some(format!("derive: {}", derive));
                    }
                }
                None
            }
            PatternMatcher::Parameter { pattern } => {
                for param in &function.parameters {
                    if self.regex_match(pattern, &param.type_annotation) {
                        return Some(format!(
                            "parameter: {}: {}",
                            param.name, param.type_annotation
                        ));
                    }
                }
                None
            }
            PatternMatcher::ReturnType { pattern } => function
                .return_type
                .as_ref()
                .filter(|rt| self.regex_match(pattern, rt))
                .map(|rt| format!("return_type: {}", rt)),
            PatternMatcher::Name { pattern } => {
                if self.regex_match(pattern, &function.name) {
                    Some(format!("name: {}", function.name))
                } else {
                    None
                }
            }
            PatternMatcher::Call { pattern } => {
                for call in &function.calls {
                    if self.regex_match(pattern, &call.name) {
                        return Some(format!("call: {}", call.name));
                    }
                }
                None
            }
            PatternMatcher::FilePath { pattern } => {
                let path_str = file_context.path.to_string_lossy();
                if self.regex_match(pattern, &path_str) {
                    Some(format!("file_path: {}", path_str))
                } else {
                    None
                }
            }
        }
    }

    /// Match regex pattern with caching
    fn regex_match(&self, pattern: &str, text: &str) -> bool {
        if !self.regex_cache.contains_key(pattern) {
            match Regex::new(pattern) {
                Ok(regex) => {
                    self.regex_cache.insert(pattern.to_string(), regex);
                }
                Err(_) => return false,
            }
        }

        if let Some(regex) = self.regex_cache.get(pattern) {
            regex.is_match(text)
        } else {
            false
        }
    }
}

/// Parse TOML configuration into framework patterns
fn parse_config_into_patterns(
    config: &toml::Value,
) -> Result<HashMap<Language, Vec<FrameworkPattern>>> {
    let mut patterns: HashMap<Language, Vec<FrameworkPattern>> = HashMap::new();

    if let Some(table) = config.as_table() {
        for (lang_key, frameworks) in table {
            let language = Language::parse(lang_key)?;

            if let Some(frameworks_table) = frameworks.as_table() {
                for (_, framework_value) in frameworks_table {
                    // Parse nested framework categories
                    if let Some(nested) = framework_value.as_table() {
                        for (_, pattern_value) in nested {
                            if let Ok(framework_pattern) =
                                toml::from_str::<FrameworkPattern>(&toml::to_string(pattern_value)?)
                            {
                                patterns
                                    .entry(language)
                                    .or_default()
                                    .push(framework_pattern);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(patterns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_caching() {
        let detector = FrameworkDetector::with_defaults();

        let pattern = "^test_.*";
        let text = "test_something";

        assert!(detector.regex_match(pattern, text));
        assert!(detector.regex_cache.contains_key(pattern));
    }

    #[test]
    fn test_pattern_matching_name() {
        let detector = FrameworkDetector::with_defaults();

        let function = FunctionAst::new("test_example".to_string());
        let file_context = FileContext::new(Language::Python, "test.py".into());

        let matcher = PatternMatcher::Name {
            pattern: "^test_.*".to_string(),
        };

        let result = detector.matches_pattern(&function, &file_context, &matcher);
        assert!(result.is_some());
        assert!(result.unwrap().contains("name: test_example"));
    }

    #[test]
    fn test_pattern_matching_decorator() {
        let detector = FrameworkDetector::with_defaults();

        let mut function = FunctionAst::new("my_fixture".to_string());
        function.decorators.push(Decorator {
            name: "@pytest.fixture".to_string(),
        });

        let file_context = FileContext::new(Language::Python, "test.py".into());

        let matcher = PatternMatcher::Decorator {
            pattern: "@pytest\\.fixture".to_string(),
        };

        let result = detector.matches_pattern(&function, &file_context, &matcher);
        assert!(result.is_some());
    }

    #[test]
    fn test_pattern_matching_import() {
        let detector = FrameworkDetector::with_defaults();

        let function = FunctionAst::new("handler".to_string());
        let mut file_context = FileContext::new(Language::Python, "app.py".into());
        file_context.add_import("from fastapi import FastAPI".to_string());

        let matcher = PatternMatcher::Import {
            pattern: "from fastapi import".to_string(),
        };

        let result = detector.matches_pattern(&function, &file_context, &matcher);
        assert!(result.is_some());
    }

    #[test]
    fn test_language_parse() {
        assert_eq!(Language::parse("rust").unwrap(), Language::Rust);
        assert_eq!(Language::parse("python").unwrap(), Language::Python);
        assert_eq!(
            Language::parse("javascript").unwrap(),
            Language::JavaScript
        );
        assert_eq!(
            Language::parse("typescript").unwrap(),
            Language::TypeScript
        );
        assert!(Language::parse("unknown").is_err());
    }
}
