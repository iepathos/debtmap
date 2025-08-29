//! Text manipulation utilities

/// Capitalizes the first character of a string
pub fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capitalize_first_empty_string() {
        assert_eq!(capitalize_first(""), "");
    }

    #[test]
    fn test_capitalize_first_single_char() {
        assert_eq!(capitalize_first("a"), "A");
        assert_eq!(capitalize_first("Z"), "Z");
    }

    #[test]
    fn test_capitalize_first_lowercase_word() {
        assert_eq!(capitalize_first("hello"), "Hello");
        assert_eq!(capitalize_first("world"), "World");
    }

    #[test]
    fn test_capitalize_first_uppercase_word() {
        assert_eq!(capitalize_first("HELLO"), "HELLO");
        assert_eq!(capitalize_first("World"), "World");
    }

    #[test]
    fn test_capitalize_first_mixed_case() {
        assert_eq!(capitalize_first("hELLO"), "HELLO");
        assert_eq!(capitalize_first("wOrLd"), "WOrLd");
    }

    #[test]
    fn test_capitalize_first_with_underscores() {
        assert_eq!(capitalize_first("hello_world"), "Hello_world");
        assert_eq!(capitalize_first("_private"), "_private");
    }

    #[test]
    fn test_capitalize_first_numbers_and_symbols() {
        assert_eq!(capitalize_first("123abc"), "123abc");
        assert_eq!(capitalize_first("!hello"), "!hello");
    }
}
