use std::path::Path;

pub fn is_test_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".t.sol"))
}

pub fn is_test_contract_name(name: &str) -> bool {
    name.ends_with("Test") || name.starts_with("Test") || name.starts_with("test")
}

pub fn is_test_function_name(name: &str) -> bool {
    let base = name.rsplit('.').next().unwrap_or(name);
    base.starts_with("test") || base.starts_with("Test")
}

pub fn uses_foundry_test_import(source: &str) -> bool {
    source.contains("forge-std/Test.sol") || source.contains("forge-std/src/Test.sol")
}

pub fn is_test_context(path: &Path, source: &str, contract_name: Option<&str>) -> bool {
    is_test_file(path)
        || uses_foundry_test_import(source)
        || contract_name.is_some_and(is_test_contract_name)
}

pub fn has_floating_pragma(source: &str) -> bool {
    source.lines().any(|line| {
        let trimmed = line.trim();
        trimmed.starts_with("pragma solidity")
            && (trimmed.contains('^') || trimmed.contains(">="))
            && !trimmed.contains('<')
    })
}

pub fn function_is_test(
    path: &Path,
    source: &str,
    contract_name: Option<&str>,
    function_name: &str,
) -> bool {
    is_test_context(path, source, contract_name) || is_test_function_name(function_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_foundry_test_file_detection() {
        assert!(is_test_file(&PathBuf::from("Token.t.sol")));
        assert!(!is_test_file(&PathBuf::from("Token.sol")));
    }

    #[test]
    fn test_floating_pragma_detection() {
        assert!(has_floating_pragma("pragma solidity ^0.8.0;"));
        assert!(!has_floating_pragma("pragma solidity 0.8.20;"));
    }
}
