
impl GlobalTypeRegistry {
    pub fn resolve_type_with_imports(&self, file: &PathBuf, name: &str) -> Option<String> {
        // First check if it's already fully qualified
        if self.types.contains_key(name) {
            return Some(name.to_string());
        }
        None
    }
}
