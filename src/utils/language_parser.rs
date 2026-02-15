use crate::core::Language;

pub fn parse_languages(languages: Option<Vec<String>>) -> Vec<Language> {
    languages
        .map(|langs| {
            langs
                .iter()
                .filter_map(|lang_str| parse_single_language(lang_str))
                .collect()
        })
        .unwrap_or_else(default_languages)
}

pub fn parse_single_language(lang_str: &str) -> Option<Language> {
    match lang_str.to_lowercase().as_str() {
        "rust" | "rs" => Some(Language::Rust),
        "python" | "py" => Some(Language::Python),
        "javascript" | "js" => Some(Language::JavaScript),
        "typescript" | "ts" => Some(Language::TypeScript),
        _ => None,
    }
}

pub fn default_languages() -> Vec<Language> {
    vec![
        Language::Rust,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
    ]
}
