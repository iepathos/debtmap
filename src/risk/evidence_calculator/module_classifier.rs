//! Pure functions for classifying module types by file path.
//!
//! This module contains stateless, pure functions that determine
//! the module type based on directory structure and filename patterns.

use crate::risk::evidence::ModuleType;
use std::path::Path;

/// Determines the module type from a file path.
///
/// Classification priority:
/// 1. Test modules (paths containing `/tests/` or `_test.rs`)
/// 2. Directory-based classification (core, api, utils, infra, db)
/// 3. Filename-based fallback (mod.rs, lib.rs, main.rs)
pub fn classify_module_type(file: &Path) -> ModuleType {
    let path_str = file.to_string_lossy();

    if is_test_module(&path_str) {
        ModuleType::Test
    } else if let Some(module_type) = classify_by_directory(&path_str) {
        module_type
    } else {
        classify_by_filename(&path_str)
    }
}

/// Pure function to check if path is a test module.
pub fn is_test_module(path_str: &str) -> bool {
    path_str.contains("/tests/") || path_str.contains("_test.rs")
}

/// Pure function to classify module by directory structure.
pub fn classify_by_directory(path_str: &str) -> Option<ModuleType> {
    match () {
        _ if path_str.contains("/core/") || path_str.contains("/domain/") => Some(ModuleType::Core),
        _ if path_str.contains("/api/") || path_str.contains("/handlers/") => Some(ModuleType::Api),
        _ if path_str.contains("/utils/") || path_str.contains("/helpers/") => {
            Some(ModuleType::Util)
        }
        _ if path_str.contains("/infra/") || path_str.contains("/db/") => {
            Some(ModuleType::Infrastructure)
        }
        _ => None,
    }
}

/// Pure function to classify module by filename.
pub fn classify_by_filename(path_str: &str) -> ModuleType {
    match () {
        _ if path_str.ends_with("mod.rs") || path_str.ends_with("lib.rs") => ModuleType::Core,
        _ if path_str.contains("main.rs") => ModuleType::Infrastructure,
        _ => ModuleType::Util,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_is_test_module() {
        assert!(is_test_module("src/tests/foo.rs"));
        assert!(is_test_module("src/module_test.rs"));
        assert!(is_test_module("path/to/tests/bar.rs"));
        assert!(!is_test_module("src/main.rs"));
        assert!(!is_test_module("src/lib.rs"));
    }

    #[test]
    fn test_classify_by_directory() {
        assert_eq!(
            classify_by_directory("src/core/logic.rs"),
            Some(ModuleType::Core)
        );
        assert_eq!(
            classify_by_directory("src/domain/model.rs"),
            Some(ModuleType::Core)
        );
        assert_eq!(
            classify_by_directory("src/api/handler.rs"),
            Some(ModuleType::Api)
        );
        assert_eq!(
            classify_by_directory("src/handlers/route.rs"),
            Some(ModuleType::Api)
        );
        assert_eq!(
            classify_by_directory("src/utils/helper.rs"),
            Some(ModuleType::Util)
        );
        assert_eq!(
            classify_by_directory("src/helpers/format.rs"),
            Some(ModuleType::Util)
        );
        assert_eq!(
            classify_by_directory("src/infra/db.rs"),
            Some(ModuleType::Infrastructure)
        );
        assert_eq!(
            classify_by_directory("src/db/connection.rs"),
            Some(ModuleType::Infrastructure)
        );
        assert_eq!(classify_by_directory("src/other/file.rs"), None);
    }

    #[test]
    fn test_classify_by_filename() {
        assert_eq!(classify_by_filename("src/mod.rs"), ModuleType::Core);
        assert_eq!(classify_by_filename("src/lib.rs"), ModuleType::Core);
        assert_eq!(
            classify_by_filename("src/main.rs"),
            ModuleType::Infrastructure
        );
        assert_eq!(
            classify_by_filename("src/foo/main.rs"),
            ModuleType::Infrastructure
        );
        assert_eq!(classify_by_filename("src/something.rs"), ModuleType::Util);
    }

    #[test]
    fn test_classify_module_type_integration() {
        // Test modules should be identified
        assert_eq!(
            classify_module_type(&PathBuf::from("src/tests/test.rs")),
            ModuleType::Test
        );

        // Core modules from directory
        assert_eq!(
            classify_module_type(&PathBuf::from("src/core/engine.rs")),
            ModuleType::Core
        );

        // API modules
        assert_eq!(
            classify_module_type(&PathBuf::from("src/api/routes.rs")),
            ModuleType::Api
        );

        // Fallback to filename classification
        assert_eq!(
            classify_module_type(&PathBuf::from("src/main.rs")),
            ModuleType::Infrastructure
        );

        // Default to Util for unknown patterns
        assert_eq!(
            classify_module_type(&PathBuf::from("src/random/file.rs")),
            ModuleType::Util
        );
    }
}
