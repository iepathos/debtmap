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

    #[test]
    fn test_new_with_special_characters_in_connection_string() {
        let special_string = "postgres://user:p@$$w0rd@localhost:5432/db?ssl=true".to_string();
        let db = Database::new(special_string.clone(), 10, 30);
        assert_eq!(db.connection_string, special_string);
    }

    #[test]
    fn test_new_with_unicode_in_connection_string() {
        let unicode_string = "postgres://用户:密码@主机/数据库".to_string();
        let db = Database::new(unicode_string.clone(), 10, 30);
        assert_eq!(db.connection_string, unicode_string);
    }

    #[test]
    fn test_new_with_very_long_connection_string() {
        let long_string = "postgres://".to_string() + &"x".repeat(1000) + "/database";
        let db = Database::new(long_string.clone(), 10, 30);
        assert_eq!(db.connection_string, long_string);
    }

    #[test]
    fn test_connection_string_accessor() {
        let db = Database::new("postgres://user:pass@host/db".to_string(), 10, 30);
        assert_eq!(db.connection_string(), "postgres://user:pass@host/db");
    }

    #[test]
    fn test_connection_string_accessor_returns_correct_value() {
        let db = Database::new("postgres://user:pass@host:5432/db".to_string(), 10, 30);
        assert_eq!(db.connection_string(), "postgres://user:pass@host:5432/db");
    }

    #[test]
    fn test_connection_string_accessor_with_empty_string() {
        let db = Database::new(String::new(), 10, 30);
        assert_eq!(db.connection_string(), "");
    }

    #[test]
    fn test_connection_string_accessor_with_special_characters() {
        let special_conn = "postgres://user%40:p%40ss@host/db?param=value&other=123";
        let db = Database::new(special_conn.to_string(), 10, 30);
        assert_eq!(db.connection_string(), special_conn);
    }

    #[test]
    fn test_pool_size_accessor() {
        let db = Database::new("postgres://localhost/test".to_string(), 25, 30);
        assert_eq!(db.pool_size(), 25);
    }

    #[test]
    fn test_pool_size_accessor_returns_correct_value() {
        let db = Database::new("postgres://localhost/test".to_string(), 25, 30);
        assert_eq!(db.pool_size(), 25);
    }

    #[test]
    fn test_pool_size_accessor_with_zero() {
        let db = Database::new("postgres://localhost/test".to_string(), 0, 30);
        assert_eq!(db.pool_size(), 0);
    }

    #[test]
    fn test_pool_size_accessor_with_max_value() {
        let db = Database::new("postgres://localhost/test".to_string(), usize::MAX, 30);
        assert_eq!(db.pool_size(), usize::MAX);
    }

    #[test]
    fn test_timeout_accessor() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 45);
        assert_eq!(db.timeout(), 45);
    }

    #[test]
    fn test_timeout_accessor_returns_correct_value() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 45);
        assert_eq!(db.timeout(), 45);
    }

    #[test]
    fn test_timeout_accessor_with_zero() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 0);
        assert_eq!(db.timeout(), 0);
    }

    #[test]
    fn test_timeout_accessor_with_max_value() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, u64::MAX);
        assert_eq!(db.timeout(), u64::MAX);
    }

    #[test]
    fn test_cache_accessor() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        let cache = db.cache();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_accessor_returns_empty_hashmap() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        assert!(db.cache().is_empty());
        assert_eq!(db.cache().len(), 0);
    }

    #[test]
    fn test_cache_accessor_returns_reference() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        let cache_ref = db.cache();
        // Verify it's a reference by checking we can use it multiple times
        assert!(cache_ref.is_empty());
        assert_eq!(cache_ref.len(), 0);
    }

    #[test]
    fn test_all_accessors_together() {
        let db = Database::new("postgres://prod/maindb".to_string(), 50, 120);

        // Test all accessor methods work correctly
        assert_eq!(db.connection_string(), "postgres://prod/maindb");
        assert_eq!(db.pool_size(), 50);
        assert_eq!(db.timeout(), 120);
        assert!(db.cache().is_empty());
    }

    #[test]
    fn test_all_accessors_work_together() {
        let conn_str = "postgres://localhost/integration";
        let pool = 50;
        let timeout = 120;
        let db = Database::new(conn_str.to_string(), pool, timeout);
        
        // Test all accessors in sequence
        assert_eq!(db.connection_string(), conn_str);
        assert_eq!(db.pool_size(), pool);
        assert_eq!(db.timeout(), timeout);
        assert!(db.cache().is_empty());
    }

    #[test]
    fn test_database_implements_debug_trait() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        let debug_str = format!("{:?}", db);
        assert!(debug_str.contains("Database"));
        assert!(debug_str.contains("connection_string"));
        assert!(debug_str.contains("pool_size"));
        assert!(debug_str.contains("timeout"));
    }

    #[test]
    fn test_database_implements_clone_trait() {
        let original = Database::new("postgres://localhost/test".to_string(), 10, 30);
        let cloned = original.clone();
        assert_eq!(original.connection_string, cloned.connection_string);
        assert_eq!(original.pool_size, cloned.pool_size);
        assert_eq!(original.timeout, cloned.timeout);
    }

    #[test]
    fn test_new_with_minimum_and_maximum_boundary_values() {
        // Test with minimum values
        let min_db = Database::new("x".to_string(), 1, 1);
        assert_eq!(min_db.pool_size, 1);
        assert_eq!(min_db.timeout, 1);

        // Test with reasonably large values
        let max_db = Database::new("x".to_string(), 10000, 86400);
        assert_eq!(max_db.pool_size, 10000);
        assert_eq!(max_db.timeout, 86400);
    }

    #[test]
    fn test_database_clone_with_accessors() {
        let db1 = Database::new("postgres://localhost/test".to_string(), 10, 30);
        let db2 = db1.clone();

        assert_eq!(db1.connection_string(), db2.connection_string());
        assert_eq!(db1.pool_size(), db2.pool_size());
        assert_eq!(db1.timeout(), db2.timeout());
    }

    #[test]
    fn test_database_debug_format_with_values() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        let debug_str = format!("{:?}", db);

        assert!(debug_str.contains("Database"));
        assert!(debug_str.contains("postgres://localhost/test"));
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("30"));
    }

    #[test]
    fn test_accessors_multiple_calls_return_same_values() {
        let db = Database::new("postgres://localhost/test".to_string(), 10, 30);
        
        // Call each accessor multiple times
        assert_eq!(db.connection_string(), db.connection_string());
        assert_eq!(db.pool_size(), db.pool_size());
        assert_eq!(db.timeout(), db.timeout());
        assert_eq!(db.cache().len(), db.cache().len());
    }
}
