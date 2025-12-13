//! Call purity classification database.
//!
//! This module maintains static databases of known pure and impure functions
//! for common Rust standard library and ecosystem crates. Used to determine
//! whether function calls can propagate taint or affect purity analysis.
//!
//! # Design
//!
//! Classification uses a two-tier approach:
//! 1. **Exact match**: Check against known function databases (e.g., `Vec::len`)
//! 2. **Pattern match**: Check method name against common patterns (e.g., `len`, `is_empty`)
//!
//! # Example
//!
//! ```ignore
//! use debtmap::analysis::data_flow::call_classification::{classify_call, CallPurity};
//!
//! assert_eq!(classify_call("Vec::len"), CallPurity::Pure);
//! assert_eq!(classify_call("Vec::push"), CallPurity::Impure);
//! assert_eq!(classify_call("my_custom_func"), CallPurity::Unknown);
//! ```

use once_cell::sync::Lazy;
use std::collections::HashSet;

/// Classification result for a function call.
///
/// Determines how taint propagates through function calls based on
/// whether the function is known to be pure, known to be impure, or unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallPurity {
    /// Known pure function - only taints through arguments
    Pure,
    /// Known impure function - always taints result
    Impure,
    /// Unknown function - use configured default
    Unknown,
}

/// Configuration for handling unknown function calls in taint analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownCallBehavior {
    /// Conservative: unknown calls always taint (current behavior, default)
    #[default]
    Conservative,
    /// Optimistic: unknown calls only taint through arguments
    Optimistic,
}

/// Database of known pure functions by qualified name.
pub static KNOWN_PURE_FUNCTIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // Numeric operations
    set.insert("std::cmp::min");
    set.insert("std::cmp::max");
    set.insert("std::cmp::Ord::cmp");
    set.insert("std::cmp::PartialOrd::partial_cmp");
    set.insert("i32::abs");
    set.insert("i64::abs");
    set.insert("f32::abs");
    set.insert("f64::abs");
    set.insert("f32::sqrt");
    set.insert("f64::sqrt");
    set.insert("f32::sin");
    set.insert("f64::sin");
    set.insert("f32::cos");
    set.insert("f64::cos");
    set.insert("f32::floor");
    set.insert("f64::floor");
    set.insert("f32::ceil");
    set.insert("f64::ceil");
    set.insert("f32::round");
    set.insert("f64::round");
    set.insert("i32::saturating_add");
    set.insert("i32::saturating_sub");
    set.insert("i64::saturating_add");
    set.insert("i64::saturating_sub");
    set.insert("i32::wrapping_add");
    set.insert("i32::wrapping_sub");
    set.insert("usize::saturating_add");
    set.insert("usize::saturating_sub");

    // Option methods
    set.insert("Option::is_some");
    set.insert("Option::is_none");
    set.insert("Option::as_ref");
    set.insert("Option::as_mut");
    set.insert("Option::unwrap_or");
    set.insert("Option::unwrap_or_else");
    set.insert("Option::unwrap_or_default");
    set.insert("Option::map");
    set.insert("Option::and_then");
    set.insert("Option::or");
    set.insert("Option::or_else");
    set.insert("Option::filter");
    set.insert("Option::flatten");
    set.insert("Option::copied");
    set.insert("Option::cloned");
    set.insert("Option::zip");
    set.insert("Option::ok_or");
    set.insert("Option::ok_or_else");

    // Result methods
    set.insert("Result::is_ok");
    set.insert("Result::is_err");
    set.insert("Result::as_ref");
    set.insert("Result::map");
    set.insert("Result::map_err");
    set.insert("Result::and_then");
    set.insert("Result::unwrap_or");
    set.insert("Result::unwrap_or_else");
    set.insert("Result::unwrap_or_default");
    set.insert("Result::ok");
    set.insert("Result::err");
    set.insert("Result::copied");
    set.insert("Result::cloned");

    // String methods
    set.insert("str::len");
    set.insert("str::is_empty");
    set.insert("str::trim");
    set.insert("str::trim_start");
    set.insert("str::trim_end");
    set.insert("str::to_lowercase");
    set.insert("str::to_uppercase");
    set.insert("str::contains");
    set.insert("str::starts_with");
    set.insert("str::ends_with");
    set.insert("str::split");
    set.insert("str::chars");
    set.insert("str::bytes");
    set.insert("str::lines");
    set.insert("str::split_whitespace");
    set.insert("str::replace");
    set.insert("str::parse");
    set.insert("String::len");
    set.insert("String::is_empty");
    set.insert("String::as_str");
    set.insert("String::as_bytes");
    set.insert("String::capacity");
    set.insert("String::chars");
    set.insert("String::bytes");

    // Vec/slice methods (pure accessors)
    set.insert("Vec::len");
    set.insert("Vec::is_empty");
    set.insert("Vec::capacity");
    set.insert("Vec::first");
    set.insert("Vec::last");
    set.insert("Vec::get");
    set.insert("Vec::contains");
    set.insert("Vec::iter");
    set.insert("Vec::as_slice");
    set.insert("Vec::binary_search");
    set.insert("Vec::starts_with");
    set.insert("Vec::ends_with");
    set.insert("[T]::len");
    set.insert("[T]::is_empty");
    set.insert("[T]::first");
    set.insert("[T]::last");
    set.insert("[T]::get");
    set.insert("[T]::contains");
    set.insert("[T]::iter");
    set.insert("[T]::split");
    set.insert("[T]::chunks");
    set.insert("[T]::windows");
    set.insert("[T]::binary_search");

    // HashMap methods (pure accessors)
    set.insert("HashMap::len");
    set.insert("HashMap::is_empty");
    set.insert("HashMap::contains_key");
    set.insert("HashMap::get");
    set.insert("HashMap::keys");
    set.insert("HashMap::values");
    set.insert("HashMap::iter");
    set.insert("HashMap::capacity");

    // HashSet methods (pure accessors)
    set.insert("HashSet::len");
    set.insert("HashSet::is_empty");
    set.insert("HashSet::contains");
    set.insert("HashSet::get");
    set.insert("HashSet::iter");
    set.insert("HashSet::capacity");
    set.insert("HashSet::is_subset");
    set.insert("HashSet::is_superset");
    set.insert("HashSet::is_disjoint");

    // BTreeMap methods (pure accessors)
    set.insert("BTreeMap::len");
    set.insert("BTreeMap::is_empty");
    set.insert("BTreeMap::contains_key");
    set.insert("BTreeMap::get");
    set.insert("BTreeMap::keys");
    set.insert("BTreeMap::values");
    set.insert("BTreeMap::iter");
    set.insert("BTreeMap::range");
    set.insert("BTreeMap::first_key_value");
    set.insert("BTreeMap::last_key_value");

    // Iterator methods (pure)
    set.insert("Iterator::count");
    set.insert("Iterator::map");
    set.insert("Iterator::filter");
    set.insert("Iterator::filter_map");
    set.insert("Iterator::flat_map");
    set.insert("Iterator::flatten");
    set.insert("Iterator::take");
    set.insert("Iterator::skip");
    set.insert("Iterator::zip");
    set.insert("Iterator::enumerate");
    set.insert("Iterator::peekable");
    set.insert("Iterator::chain");
    set.insert("Iterator::fold");
    set.insert("Iterator::reduce");
    set.insert("Iterator::all");
    set.insert("Iterator::any");
    set.insert("Iterator::find");
    set.insert("Iterator::position");
    set.insert("Iterator::sum");
    set.insert("Iterator::product");
    set.insert("Iterator::collect");
    set.insert("Iterator::nth");
    set.insert("Iterator::last");
    set.insert("Iterator::min");
    set.insert("Iterator::max");
    set.insert("Iterator::min_by");
    set.insert("Iterator::max_by");
    set.insert("Iterator::min_by_key");
    set.insert("Iterator::max_by_key");
    set.insert("Iterator::rev");
    set.insert("Iterator::cloned");
    set.insert("Iterator::copied");
    set.insert("Iterator::by_ref");
    set.insert("Iterator::step_by");
    set.insert("Iterator::take_while");
    set.insert("Iterator::skip_while");
    set.insert("Iterator::partition");
    set.insert("Iterator::unzip");
    set.insert("Iterator::inspect");
    set.insert("Iterator::fuse");
    set.insert("Iterator::cycle");

    // Clone/Copy
    set.insert("Clone::clone");
    set.insert("ToOwned::to_owned");

    // Display/Debug (pure - just formatting)
    set.insert("Display::fmt");
    set.insert("Debug::fmt");
    set.insert("ToString::to_string");

    // Conversion traits
    set.insert("From::from");
    set.insert("Into::into");
    set.insert("TryFrom::try_from");
    set.insert("TryInto::try_into");
    set.insert("AsRef::as_ref");
    set.insert("AsMut::as_mut");
    set.insert("Deref::deref");
    set.insert("DerefMut::deref_mut");
    set.insert("Borrow::borrow");
    set.insert("BorrowMut::borrow_mut");

    // Default
    set.insert("Default::default");

    // Comparison
    set.insert("PartialEq::eq");
    set.insert("PartialEq::ne");
    set.insert("Eq::eq");
    set.insert("PartialOrd::partial_cmp");
    set.insert("PartialOrd::lt");
    set.insert("PartialOrd::le");
    set.insert("PartialOrd::gt");
    set.insert("PartialOrd::ge");
    set.insert("Ord::cmp");
    set.insert("Ord::max");
    set.insert("Ord::min");
    set.insert("Ord::clamp");

    // Hash
    set.insert("Hash::hash");

    // Index
    set.insert("Index::index");
    set.insert("IndexMut::index_mut");

    // Path methods
    set.insert("Path::exists");
    set.insert("Path::is_file");
    set.insert("Path::is_dir");
    set.insert("Path::extension");
    set.insert("Path::file_name");
    set.insert("Path::file_stem");
    set.insert("Path::parent");
    set.insert("Path::join");
    set.insert("Path::with_extension");
    set.insert("Path::to_str");
    set.insert("Path::to_string_lossy");
    set.insert("Path::display");
    set.insert("Path::canonicalize");
    set.insert("Path::components");
    set.insert("PathBuf::as_path");
    set.insert("PathBuf::push");
    set.insert("PathBuf::set_extension");

    set
});

/// Known impure functions (side effects).
pub static KNOWN_IMPURE_FUNCTIONS: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    let mut set = HashSet::new();

    // I/O
    set.insert("std::io::Read::read");
    set.insert("std::io::Read::read_to_string");
    set.insert("std::io::Read::read_to_end");
    set.insert("std::io::Write::write");
    set.insert("std::io::Write::write_all");
    set.insert("std::io::Write::flush");
    set.insert("std::fs::read");
    set.insert("std::fs::read_to_string");
    set.insert("std::fs::write");
    set.insert("std::fs::File::open");
    set.insert("std::fs::File::create");
    set.insert("std::fs::remove_file");
    set.insert("std::fs::remove_dir");
    set.insert("std::fs::remove_dir_all");
    set.insert("std::fs::create_dir");
    set.insert("std::fs::create_dir_all");
    set.insert("std::fs::rename");
    set.insert("std::fs::copy");
    set.insert("std::fs::metadata");
    set.insert("std::fs::read_dir");
    set.insert("println");
    set.insert("print");
    set.insert("eprintln");
    set.insert("eprint");
    set.insert("dbg");

    // Network
    set.insert("std::net::TcpStream::connect");
    set.insert("std::net::TcpListener::bind");
    set.insert("std::net::UdpSocket::bind");
    set.insert("std::net::UdpSocket::send");
    set.insert("std::net::UdpSocket::recv");

    // Random/Time
    set.insert("rand::random");
    set.insert("rand::thread_rng");
    set.insert("rand::Rng::gen");
    set.insert("std::time::Instant::now");
    set.insert("std::time::SystemTime::now");

    // Mutation methods (Vec)
    set.insert("Vec::push");
    set.insert("Vec::pop");
    set.insert("Vec::insert");
    set.insert("Vec::remove");
    set.insert("Vec::clear");
    set.insert("Vec::truncate");
    set.insert("Vec::extend");
    set.insert("Vec::append");
    set.insert("Vec::drain");
    set.insert("Vec::retain");
    set.insert("Vec::resize");
    set.insert("Vec::swap_remove");
    set.insert("Vec::dedup");
    set.insert("Vec::sort");
    set.insert("Vec::sort_by");
    set.insert("Vec::sort_by_key");
    set.insert("Vec::reverse");

    // Mutation methods (HashMap)
    set.insert("HashMap::insert");
    set.insert("HashMap::remove");
    set.insert("HashMap::clear");
    set.insert("HashMap::drain");
    set.insert("HashMap::retain");
    set.insert("HashMap::entry");

    // Mutation methods (HashSet)
    set.insert("HashSet::insert");
    set.insert("HashSet::remove");
    set.insert("HashSet::clear");
    set.insert("HashSet::drain");
    set.insert("HashSet::retain");

    // Mutation methods (BTreeMap)
    set.insert("BTreeMap::insert");
    set.insert("BTreeMap::remove");
    set.insert("BTreeMap::clear");
    set.insert("BTreeMap::pop_first");
    set.insert("BTreeMap::pop_last");

    // String mutation
    set.insert("String::push");
    set.insert("String::push_str");
    set.insert("String::pop");
    set.insert("String::insert");
    set.insert("String::insert_str");
    set.insert("String::remove");
    set.insert("String::clear");
    set.insert("String::truncate");
    set.insert("String::retain");
    set.insert("String::drain");

    // RefCell/Cell
    set.insert("RefCell::borrow_mut");
    set.insert("RefCell::replace");
    set.insert("RefCell::swap");
    set.insert("Cell::set");
    set.insert("Cell::replace");
    set.insert("Cell::swap");

    // Mutex/RwLock
    set.insert("Mutex::lock");
    set.insert("RwLock::write");
    set.insert("RwLock::read");

    // Threading
    set.insert("std::thread::spawn");
    set.insert("std::thread::sleep");
    set.insert("JoinHandle::join");

    // Channels
    set.insert("Sender::send");
    set.insert("Receiver::recv");
    set.insert("Receiver::try_recv");

    // Environment
    set.insert("std::env::var");
    set.insert("std::env::set_var");
    set.insert("std::env::remove_var");
    set.insert("std::env::args");
    set.insert("std::env::current_dir");
    set.insert("std::env::set_current_dir");

    // Process
    set.insert("std::process::Command::new");
    set.insert("std::process::Command::spawn");
    set.insert("std::process::Command::output");
    set.insert("std::process::exit");
    set.insert("std::process::abort");

    set
});

/// Pure method name patterns for pattern-based matching.
pub const PURE_METHOD_PATTERNS: &[&str] = &[
    "len",
    "is_empty",
    "is_some",
    "is_none",
    "is_ok",
    "is_err",
    "as_ref",
    "as_mut",
    "as_str",
    "as_slice",
    "as_bytes",
    "as_path",
    "get",
    "first",
    "last",
    "contains",
    "contains_key",
    "clone",
    "to_owned",
    "to_string",
    "to_lowercase",
    "to_uppercase",
    "map",
    "filter",
    "and_then",
    "or_else",
    "unwrap_or",
    "unwrap_or_default",
    "unwrap_or_else",
    "iter",
    "into_iter",
    "chars",
    "bytes",
    "lines",
    "trim",
    "trim_start",
    "trim_end",
    "abs",
    "sqrt",
    "sin",
    "cos",
    "floor",
    "ceil",
    "round",
    "min",
    "max",
    "clamp",
    "cmp",
    "partial_cmp",
    "eq",
    "ne",
    "lt",
    "le",
    "gt",
    "ge",
    "copied",
    "cloned",
    "flatten",
    "zip",
    "enumerate",
    "rev",
    "take",
    "skip",
    "fold",
    "reduce",
    "all",
    "any",
    "find",
    "position",
    "sum",
    "product",
    "collect",
    "count",
    "nth",
    "split",
    "chunks",
    "windows",
    "starts_with",
    "ends_with",
    "binary_search",
    "is_subset",
    "is_superset",
    "is_disjoint",
    "capacity",
    "keys",
    "values",
    "from",
    "into",
    "default",
    "hash",
    "index",
    "deref",
    "borrow",
    "display",
    "fmt",
    "parse",
];

/// Impure method name patterns for pattern-based matching.
pub const IMPURE_METHOD_PATTERNS: &[&str] = &[
    "push",
    "pop",
    "insert",
    "remove",
    "clear",
    "truncate",
    "extend",
    "append",
    "drain",
    "retain",
    "resize",
    "swap_remove",
    "dedup",
    "sort",
    "sort_by",
    "sort_by_key",
    "reverse",
    "read",
    "read_to_string",
    "read_to_end",
    "write",
    "write_all",
    "flush",
    "connect",
    "bind",
    "listen",
    "accept",
    "send",
    "recv",
    "spawn",
    "join",
    "sleep",
    "lock",
    "unlock",
    "now",
    "elapsed",
    "random",
    "gen",
    "shuffle",
    "set",
    "replace",
    "swap",
    "borrow_mut",
    "entry",
    "pop_first",
    "pop_last",
];

/// Classify a function call by its name.
///
/// Checks against known pure/impure function databases and method name patterns
/// to determine how taint should propagate through the call.
///
/// # Arguments
///
/// * `func_name` - The function name, possibly qualified (e.g., "Vec::len", "std::fs::read")
///
/// # Returns
///
/// * `CallPurity::Pure` - Function is known to be pure (no side effects)
/// * `CallPurity::Impure` - Function is known to be impure (has side effects)
/// * `CallPurity::Unknown` - Function purity is unknown
///
/// # Example
///
/// ```ignore
/// assert_eq!(classify_call("Vec::len"), CallPurity::Pure);
/// assert_eq!(classify_call("Vec::push"), CallPurity::Impure);
/// assert_eq!(classify_call("my_custom_func"), CallPurity::Unknown);
/// ```
pub fn classify_call(func_name: &str) -> CallPurity {
    // Check known pure functions first
    if is_known_pure(func_name) {
        return CallPurity::Pure;
    }

    // Check known impure functions
    if is_known_impure(func_name) {
        return CallPurity::Impure;
    }

    // Unknown
    CallPurity::Unknown
}

/// Check if a function is known to be pure.
pub fn is_known_pure(func_name: &str) -> bool {
    // Exact match
    if KNOWN_PURE_FUNCTIONS.contains(func_name) {
        return true;
    }

    // Method name match (for unqualified calls)
    let method_name = func_name.rsplit("::").next().unwrap_or(func_name);
    PURE_METHOD_PATTERNS.contains(&method_name)
}

/// Check if a function is known to be impure.
pub fn is_known_impure(func_name: &str) -> bool {
    // Exact match
    if KNOWN_IMPURE_FUNCTIONS.contains(func_name) {
        return true;
    }

    // Method name match
    let method_name = func_name.rsplit("::").next().unwrap_or(func_name);
    IMPURE_METHOD_PATTERNS.contains(&method_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_std_pure_functions() {
        assert_eq!(classify_call("Vec::len"), CallPurity::Pure);
        assert_eq!(classify_call("Option::map"), CallPurity::Pure);
        assert_eq!(classify_call("Option::is_some"), CallPurity::Pure);
        assert_eq!(classify_call("Result::is_ok"), CallPurity::Pure);
        assert_eq!(classify_call("Iterator::filter"), CallPurity::Pure);
        assert_eq!(classify_call("str::trim"), CallPurity::Pure);
        assert_eq!(classify_call("Clone::clone"), CallPurity::Pure);
        assert_eq!(classify_call("Default::default"), CallPurity::Pure);
    }

    #[test]
    fn test_classify_std_impure_functions() {
        assert_eq!(classify_call("Vec::push"), CallPurity::Impure);
        assert_eq!(classify_call("Vec::pop"), CallPurity::Impure);
        assert_eq!(classify_call("HashMap::insert"), CallPurity::Impure);
        assert_eq!(classify_call("std::fs::read"), CallPurity::Impure);
        assert_eq!(classify_call("println"), CallPurity::Impure);
        assert_eq!(classify_call("std::time::Instant::now"), CallPurity::Impure);
    }

    #[test]
    fn test_classify_unknown_functions() {
        assert_eq!(classify_call("my_custom_func"), CallPurity::Unknown);
        assert_eq!(
            classify_call("unknown_module::mystery_func"),
            CallPurity::Unknown
        );
        assert_eq!(classify_call("foo"), CallPurity::Unknown);
    }

    #[test]
    fn test_pure_method_pattern_matching() {
        assert_eq!(classify_call("len"), CallPurity::Pure);
        assert_eq!(classify_call("is_empty"), CallPurity::Pure);
        assert_eq!(classify_call("clone"), CallPurity::Pure);
        assert_eq!(classify_call("to_string"), CallPurity::Pure);
        assert_eq!(classify_call("iter"), CallPurity::Pure);
        assert_eq!(classify_call("map"), CallPurity::Pure);
        assert_eq!(classify_call("filter"), CallPurity::Pure);
        assert_eq!(classify_call("contains"), CallPurity::Pure);
    }

    #[test]
    fn test_impure_method_pattern_matching() {
        assert_eq!(classify_call("push"), CallPurity::Impure);
        assert_eq!(classify_call("pop"), CallPurity::Impure);
        assert_eq!(classify_call("insert"), CallPurity::Impure);
        assert_eq!(classify_call("remove"), CallPurity::Impure);
        assert_eq!(classify_call("clear"), CallPurity::Impure);
        assert_eq!(classify_call("write"), CallPurity::Impure);
        assert_eq!(classify_call("now"), CallPurity::Impure);
    }

    #[test]
    fn test_qualified_method_pattern_matching() {
        assert_eq!(classify_call("MyType::len"), CallPurity::Pure);
        assert_eq!(classify_call("custom::module::is_empty"), CallPurity::Pure);
        assert_eq!(classify_call("MyVec::push"), CallPurity::Impure);
        assert_eq!(classify_call("custom::module::clear"), CallPurity::Impure);
    }

    #[test]
    fn test_call_purity_enum_equality() {
        assert_eq!(CallPurity::Pure, CallPurity::Pure);
        assert_eq!(CallPurity::Impure, CallPurity::Impure);
        assert_eq!(CallPurity::Unknown, CallPurity::Unknown);
        assert_ne!(CallPurity::Pure, CallPurity::Impure);
        assert_ne!(CallPurity::Pure, CallPurity::Unknown);
        assert_ne!(CallPurity::Impure, CallPurity::Unknown);
    }

    #[test]
    fn test_unknown_call_behavior_default() {
        let default_behavior = UnknownCallBehavior::default();
        assert_eq!(default_behavior, UnknownCallBehavior::Conservative);
    }

    #[test]
    fn test_classification_performance() {
        use std::time::Instant;

        let funcs = vec![
            "Vec::len",
            "Option::map",
            "std::fs::read",
            "unknown_func",
            "Iterator::collect",
            "HashMap::insert",
            "String::trim",
            "push",
            "clone",
            "my_custom_function",
        ];

        let start = Instant::now();
        for _ in 0..10000 {
            for func in &funcs {
                let _ = classify_call(func);
            }
        }
        let elapsed = start.elapsed();

        assert!(
            elapsed.as_millis() < 500,
            "Classification took {:?}, expected <500ms (spec: <0.5ms/lookup)",
            elapsed
        );
    }

    #[test]
    fn test_known_pure_function_count() {
        let count = KNOWN_PURE_FUNCTIONS.len();
        assert!(
            count >= 100,
            "Should have at least 100 known pure functions, got {}",
            count
        );
    }

    #[test]
    fn test_pure_patterns_completeness() {
        let patterns = [
            "len", "is_empty", "is_some", "is_none", "is_ok", "is_err", "get", "first", "last",
            "contains", "clone", "iter", "map", "filter",
        ];

        for pattern in patterns {
            assert_eq!(
                classify_call(pattern),
                CallPurity::Pure,
                "{} should be classified as pure",
                pattern
            );
        }
    }

    #[test]
    fn test_impure_patterns_completeness() {
        let patterns = [
            "push", "pop", "insert", "remove", "clear", "write", "read", "now",
        ];

        for pattern in patterns {
            assert_eq!(
                classify_call(pattern),
                CallPurity::Impure,
                "{} should be classified as impure",
                pattern
            );
        }
    }
}
