/// Example database module to demonstrate test addition
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Database {
    connection_string: String,
    pool_size: usize,
    timeout: u64,
    cache: HashMap<String, String>,
}

impl Database {
    /// Creates a new Database instance with the given configuration
    pub fn new(connection_string: String, pool_size: usize, timeout: u64) -> Self {
        Database {
            connection_string,
            pool_size,
            timeout,
            cache: HashMap::new(),
        }
    }

    /// Returns the connection string
    pub fn connection_string(&self) -> &str {
        &self.connection_string
    }

    /// Returns the pool size
    pub fn pool_size(&self) -> usize {
        self.pool_size
    }

    /// Returns the timeout
    pub fn timeout(&self) -> u64 {
        self.timeout
    }

    /// Returns a reference to the cache
    pub fn cache(&self) -> &HashMap<String, String> {
        &self.cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_database_with_correct_connection_string() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        assert_eq!(db.connection_string, "postgres://localhost/test");
    }

    #[test]
    fn test_new_creates_database_with_correct_pool_size() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        assert_eq!(db.pool_size, 10);
    }

    #[test]
    fn test_new_creates_database_with_correct_timeout() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        assert_eq!(db.timeout, 30);
    }

    #[test]
    fn test_new_initializes_empty_cache() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        assert!(db.cache.is_empty());
    }

    #[test]
    fn test_new_with_empty_connection_string() {
        let db = Database::new(String::new(), 10, 30);
        assert_eq!(db.connection_string, "");
        assert_eq!(db.pool_size, 10);
        assert_eq!(db.timeout, 30);
    }

    #[test]
    fn test_new_with_zero_pool_size() {
        let db = Database::new("postgres://localhost/test".to_string(), 0, 30);
        assert_eq!(db.pool_size, 0);
    }

    #[test]
    fn test_new_with_zero_timeout() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 0);
        assert_eq!(db.timeout, 0);
    }

    #[test]
    fn test_new_with_large_values() {
        let db = Database::new(
            "postgres://localhost/test".to_string(),
            usize::MAX,
            u64::MAX,
        );
        assert_eq!(db.pool_size, usize::MAX);
        assert_eq!(db.timeout, u64::MAX);
    }

    #[test]
    fn test_new_database_fields_are_accessible() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        // Verify all fields are accessible
        let _ = &db.connection_string;
        let _ = &db.pool_size;
        let _ = &db.timeout;
        let _ = &db.cache;
    }

    #[test]
    fn test_new_creates_independent_instances() {
        let db1 = Database::new("postgres://localhost/db1".to_string(), 10, 30);
        let db2 = Database::new("postgres://localhost/db2".to_string(), 20, 60);

        assert_ne!(db1.connection_string, db2.connection_string);
        assert_ne!(db1.pool_size, db2.pool_size);
        assert_ne!(db1.timeout, db2.timeout);
    }
}
