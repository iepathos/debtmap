// Example high-complexity function that needs refactoring
// Simulates the debt item from the JSON with cyclomatic complexity 11

use std::path::Path;

/// Refactored function using pattern consolidation
/// Reduced cyclomatic complexity from 11 to <10
pub fn classify_file_operation(path: &Path, operation: &str) -> FileOperationType {
    let path_str = path.to_string_lossy();
    
    // Extract pure classification logic
    let path_type = classify_path_type(&path_str);
    
    // Use pattern matching for cleaner logic
    match (path_type, operation) {
        (PathType::Temp, "write" | "create") => FileOperationType::TempWrite,
        (PathType::Temp, "read") => FileOperationType::TempRead,
        (PathType::Temp, _) => FileOperationType::TempOther,
        (PathType::Cache, "write") => FileOperationType::CacheWrite,
        (PathType::Cache, "read") => FileOperationType::CacheRead,
        (PathType::Cache, _) => FileOperationType::CacheOther,
        (PathType::Log, "append") => FileOperationType::LogAppend,
        (PathType::Log, _) => FileOperationType::LogOther,
        (PathType::Config, _) => FileOperationType::Configuration,
        (PathType::Regular, _) => FileOperationType::Regular,
    }
}

/// Pure function for path type classification
/// Extracted to reduce complexity and improve testability
fn classify_path_type(path_str: &str) -> PathType {
    match () {
        _ if path_str.contains("temp") || path_str.contains("tmp") => PathType::Temp,
        _ if path_str.contains("cache") => PathType::Cache,
        _ if path_str.contains("log") => PathType::Log,
        _ if path_str.contains("config") || path_str.contains("settings") => PathType::Config,
        _ => PathType::Regular,
    }
}

#[derive(Debug, PartialEq)]
enum PathType {
    Temp,
    Cache,
    Log,
    Config,
    Regular,
}

#[derive(Debug, PartialEq)]
pub enum FileOperationType {
    TempWrite,
    TempRead,
    TempOther,
    CacheWrite,
    CacheRead,
    CacheOther,
    LogAppend,
    LogOther,
    Configuration,
    Regular,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // Test the pure classification function
    #[test]
    fn test_classify_path_type() {
        assert_eq!(classify_path_type("/tmp/file.txt"), PathType::Temp);
        assert_eq!(classify_path_type("/var/temp/data"), PathType::Temp);
        assert_eq!(classify_path_type("/var/cache/app"), PathType::Cache);
        assert_eq!(classify_path_type("/var/log/system.log"), PathType::Log);
        assert_eq!(classify_path_type("/etc/config/app.conf"), PathType::Config);
        assert_eq!(classify_path_type("/home/user/settings.ini"), PathType::Config);
        assert_eq!(classify_path_type("/home/user/document.txt"), PathType::Regular);
    }

    #[test]
    fn test_temp_operations() {
        let path = Path::new("/tmp/file.txt");
        assert_eq!(classify_file_operation(path, "write"), FileOperationType::TempWrite);
        assert_eq!(classify_file_operation(path, "create"), FileOperationType::TempWrite);
        assert_eq!(classify_file_operation(path, "read"), FileOperationType::TempRead);
        assert_eq!(classify_file_operation(path, "delete"), FileOperationType::TempOther);
    }

    #[test]
    fn test_cache_operations() {
        let path = Path::new("/var/cache/data");
        assert_eq!(classify_file_operation(path, "write"), FileOperationType::CacheWrite);
        assert_eq!(classify_file_operation(path, "read"), FileOperationType::CacheRead);
        assert_eq!(classify_file_operation(path, "invalidate"), FileOperationType::CacheOther);
    }

    #[test]
    fn test_log_operations() {
        let path = Path::new("/var/log/app.log");
        assert_eq!(classify_file_operation(path, "append"), FileOperationType::LogAppend);
        assert_eq!(classify_file_operation(path, "rotate"), FileOperationType::LogOther);
        assert_eq!(classify_file_operation(path, "read"), FileOperationType::LogOther);
    }

    #[test]
    fn test_config_operations() {
        let path = Path::new("/etc/config/app.conf");
        assert_eq!(classify_file_operation(path, "read"), FileOperationType::Configuration);
        assert_eq!(classify_file_operation(path, "write"), FileOperationType::Configuration);
        
        let settings_path = Path::new("/home/user/settings.json");
        assert_eq!(classify_file_operation(settings_path, "update"), FileOperationType::Configuration);
    }

    #[test]
    fn test_regular_file_operations() {
        let path = Path::new("/home/user/document.txt");
        assert_eq!(classify_file_operation(path, "read"), FileOperationType::Regular);
        assert_eq!(classify_file_operation(path, "write"), FileOperationType::Regular);
    }
}