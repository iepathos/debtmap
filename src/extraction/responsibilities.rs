//! Pure responsibility grouping for extracted functions.
//!
//! This module normalizes function identity and language-shaped naming patterns
//! before downstream adapters consume responsibility groups.

use crate::extraction::types::ExtractedFunctionData;
use crate::organization::god_object::classifier::infer_responsibility_with_confidence;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResponsibilityGroups {
    pub groups: HashMap<String, Vec<String>>,
    pub unique_function_names: Vec<String>,
}

impl FileResponsibilityGroups {
    pub fn function_count(&self) -> usize {
        self.unique_function_names.len()
    }

    pub fn responsibility_count(&self) -> usize {
        self.groups.len()
    }

    pub fn responsibility_method_counts(&self) -> HashMap<String, usize> {
        self.groups
            .iter()
            .map(|(name, methods)| (name.clone(), methods.len()))
            .collect()
    }

    pub fn sorted_responsibility_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.groups.keys().cloned().collect();
        names.sort();
        names
    }
}

pub fn file_responsibility_groups(functions: &[ExtractedFunctionData]) -> FileResponsibilityGroups {
    let mut seen = HashSet::new();

    let mut grouped = functions.iter().fold(
        FileResponsibilityGroups {
            groups: HashMap::new(),
            unique_function_names: Vec::new(),
        },
        |mut grouped, func| {
            let display_name = function_display_name(func);
            if seen.insert(display_name.clone()) {
                grouped.unique_function_names.push(display_name.clone());
                if let Some(responsibility) = file_level_responsibility(func) {
                    grouped
                        .groups
                        .entry(responsibility)
                        .or_default()
                        .push(display_name);
                }
            }

            grouped
        },
    );

    grouped.unique_function_names.sort();
    grouped
}

fn function_display_name(func: &ExtractedFunctionData) -> String {
    if func.qualified_name.is_empty() {
        func.name.clone()
    } else {
        func.qualified_name.clone()
    }
}

fn file_level_responsibility(func: &ExtractedFunctionData) -> Option<String> {
    if is_generic_route_method(&func.name) {
        return owner_domain_from_qualified_name(&func.qualified_name);
    }

    let result = infer_responsibility_with_confidence(&func.name, None);
    result
        .category
        .filter(|category| category != "unclassified")
        .or_else(|| owner_domain_from_qualified_name(&func.qualified_name))
}

fn owner_domain_from_qualified_name(qualified_name: &str) -> Option<String> {
    let owner = qualified_name
        .rsplit_once('.')
        .map(|(owner, _)| owner)
        .or_else(|| qualified_name.rsplit_once("::").map(|(owner, _)| owner))?;
    let owner_name = owner.rsplit(['.', ':']).next().unwrap_or(owner);
    let domain = split_camel_words(strip_role_suffix(owner_name));

    (!domain.is_empty()).then_some(domain)
}

fn is_generic_route_method(name: &str) -> bool {
    matches!(
        name,
        "get" | "post" | "put" | "patch" | "delete" | "options" | "head"
    )
}

fn strip_role_suffix(name: &str) -> &str {
    const SUFFIXES: &[&str] = &[
        "Handler",
        "Controller",
        "ViewSet",
        "View",
        "Service",
        "Repository",
        "Manager",
        "List",
        "Version",
    ];

    let mut stripped = name;
    loop {
        let Some(next) = SUFFIXES
            .iter()
            .find_map(|suffix| stripped.strip_suffix(suffix))
            .filter(|next| !next.is_empty())
        else {
            return stripped;
        };
        stripped = next;
    }
}

fn split_camel_words(name: &str) -> String {
    let words = name.chars().fold(Vec::new(), |mut words, ch| {
        if ch == '_' || ch == '-' {
            push_word_boundary(&mut words);
            return words;
        }

        if ch.is_uppercase() && words.last().is_some_and(|word| !word.is_empty()) {
            push_word_boundary(&mut words);
        }

        if words.is_empty() {
            words.push(String::new());
        }
        words.last_mut().expect("word slot exists").push(ch);
        words
    });

    words
        .into_iter()
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn push_word_boundary(words: &mut Vec<String>) {
    if words.last().is_none_or(|word| !word.is_empty()) {
        words.push(String::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extraction::types::PurityAnalysisData;

    fn function(name: &str, qualified_name: &str) -> ExtractedFunctionData {
        ExtractedFunctionData {
            name: name.to_string(),
            qualified_name: qualified_name.to_string(),
            line: 1,
            end_line: 2,
            length: 2,
            cyclomatic: 1,
            cognitive: 1,
            nesting: 0,
            entropy_score: None,
            purity_analysis: PurityAnalysisData::pure(),
            io_operations: vec![],
            parameter_names: vec![],
            transformation_patterns: vec![],
            calls: vec![],
            is_test: false,
            is_async: false,
            visibility: Some("pub".to_string()),
            is_trait_method: false,
            in_test_module: false,
        }
    }

    #[test]
    fn route_methods_use_owner_domain() {
        let groups = file_responsibility_groups(&[
            function("get", "CharacterListHandler.get"),
            function("post", "CharacterListHandler.post"),
            function("delete", "CharacterVersionHandler.delete"),
        ]);

        assert_eq!(groups.function_count(), 3);
        assert_eq!(
            groups.groups.get("Character"),
            Some(&vec![
                "CharacterListHandler.get".to_string(),
                "CharacterListHandler.post".to_string(),
                "CharacterVersionHandler.delete".to_string()
            ])
        );
    }

    #[test]
    fn repeated_extraction_entries_are_deduplicated() {
        let groups = file_responsibility_groups(&[
            function("get", "CityHandler.get"),
            function("get", "CityHandler.get"),
        ]);

        assert_eq!(groups.function_count(), 1);
        assert_eq!(
            groups.groups.get("City"),
            Some(&vec!["CityHandler.get".to_string()])
        );
    }

    #[test]
    fn named_functions_use_behavioral_classifier() {
        let groups =
            file_responsibility_groups(&[function("validate_payload", "validate_payload")]);

        assert_eq!(
            groups.groups.get("Validation"),
            Some(&vec!["validate_payload".to_string()])
        );
    }

    #[test]
    fn owner_domain_splits_compound_suffixes() {
        assert_eq!(
            owner_domain_from_qualified_name("WorldBackgroundGenerationHandler.post"),
            Some("World Background Generation".to_string())
        );
    }

    #[test]
    fn function_display_names_are_sorted_for_stable_weighting() {
        let groups = file_responsibility_groups(&[
            function("post", "WorldHandler.post"),
            function("get", "CharacterHandler.get"),
        ]);

        assert_eq!(
            groups.unique_function_names,
            vec![
                "CharacterHandler.get".to_string(),
                "WorldHandler.post".to_string()
            ]
        );
    }
}
