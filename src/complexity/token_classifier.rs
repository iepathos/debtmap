use std::collections::HashMap;

/// Represents different types of variables for classification
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VarType {
    Iterator,      // Loop iterators (i, j, k, iter)
    Counter,       // Counting variables
    Temporary,     // temp, tmp, result
    Configuration, // config, settings, options
    Resource,      // file, conn, db, client
    Data,          // data, value, item
    Other,         // Everything else
}

/// Represents different types of field access patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AccessType {
    Getter,     // Simple field access
    Setter,     // Field assignment
    Chained,    // a.b.c pattern
    Collection, // Array/map access
}

/// Represents different types of method calls
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CallType {
    Getter,      // get_*, *_ref, as_*
    Setter,      // set_*, with_*
    Validator,   // is_*, has_*, can_*, should_*
    Converter,   // to_*, into_*, from_*
    IO,          // read, write, send, receive
    ErrorHandle, // unwrap, expect, map_err
    Collection,  // push, pop, insert, remove
    External,    // Calls to external crates
    Other,       // Everything else
}

/// Represents different control flow constructs
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FlowType {
    If,
    Match,
    Loop,
    While,
    For,
    Return,
    Break,
    Continue,
}

/// Represents error handling patterns
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorType {
    Result,
    Option,
    Unwrap,
    Expect,
    QuestionMark,
    MapErr,
    AndThen,
    OrElse,
}

/// Represents collection operations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CollectionOp {
    Iteration,   // iter, into_iter
    Mapping,     // map, filter_map
    Filtering,   // filter, take_while
    Aggregation, // fold, reduce, collect
    Access,      // get, contains
    Mutation,    // push, insert, remove
}

/// Represents different types of literals
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LiteralCategory {
    Numeric,
    String,
    Boolean,
    Char,
    Null,
}

/// Main token classification enum
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenClass {
    LocalVar(VarType),
    FieldAccess(AccessType),
    MethodCall(CallType),
    ExternalAPI(String), // Module/crate name
    ControlFlow(FlowType),
    ErrorHandling(ErrorType),
    Collection(CollectionOp),
    Literal(LiteralCategory),
    Keyword(String),
    Operator(String),
    Unknown(String),
}

/// Context information for token classification
#[derive(Debug, Clone)]
pub struct TokenContext {
    pub is_method_call: bool,
    pub is_field_access: bool,
    pub is_external: bool,
    pub scope_depth: usize,
    pub parent_node_type: NodeType,
}

/// Represents the type of parent AST node
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    Function,
    Method,
    Closure,
    Block,
    Expression,
    Statement,
    Pattern,
    Type,
}

/// Configuration for token classification
#[derive(Debug, Clone)]
pub struct ClassificationConfig {
    pub enabled: bool,
    pub weights: HashMap<TokenClass, f64>,
    pub cache_size: usize,
}

impl Default for ClassificationConfig {
    fn default() -> Self {
        let mut weights = HashMap::new();

        // Local variables - lower weight for common patterns
        weights.insert(TokenClass::LocalVar(VarType::Iterator), 0.1);
        weights.insert(TokenClass::LocalVar(VarType::Counter), 0.2);
        weights.insert(TokenClass::LocalVar(VarType::Temporary), 0.3);
        weights.insert(TokenClass::LocalVar(VarType::Configuration), 0.5);
        weights.insert(TokenClass::LocalVar(VarType::Resource), 0.7);
        weights.insert(TokenClass::LocalVar(VarType::Data), 0.5);
        weights.insert(TokenClass::LocalVar(VarType::Other), 0.4);

        // Field access - moderate weight
        weights.insert(TokenClass::FieldAccess(AccessType::Getter), 0.3);
        weights.insert(TokenClass::FieldAccess(AccessType::Setter), 0.4);
        weights.insert(TokenClass::FieldAccess(AccessType::Chained), 0.6);
        weights.insert(TokenClass::FieldAccess(AccessType::Collection), 0.5);

        // Method calls - varied by type
        weights.insert(TokenClass::MethodCall(CallType::Getter), 0.2);
        weights.insert(TokenClass::MethodCall(CallType::Setter), 0.3);
        weights.insert(TokenClass::MethodCall(CallType::Validator), 0.4);
        weights.insert(TokenClass::MethodCall(CallType::Converter), 0.5);
        weights.insert(TokenClass::MethodCall(CallType::IO), 0.9);
        weights.insert(TokenClass::MethodCall(CallType::ErrorHandle), 0.7);
        weights.insert(TokenClass::MethodCall(CallType::Collection), 0.4);
        weights.insert(TokenClass::MethodCall(CallType::External), 1.0);
        weights.insert(TokenClass::MethodCall(CallType::Other), 0.6);

        // Control flow - standard weight
        weights.insert(TokenClass::ControlFlow(FlowType::If), 0.5);
        weights.insert(TokenClass::ControlFlow(FlowType::Match), 0.6);
        weights.insert(TokenClass::ControlFlow(FlowType::Loop), 0.7);
        weights.insert(TokenClass::ControlFlow(FlowType::While), 0.7);
        weights.insert(TokenClass::ControlFlow(FlowType::For), 0.6);
        weights.insert(TokenClass::ControlFlow(FlowType::Return), 0.3);
        weights.insert(TokenClass::ControlFlow(FlowType::Break), 0.4);
        weights.insert(TokenClass::ControlFlow(FlowType::Continue), 0.4);

        // Error handling - higher weight
        weights.insert(TokenClass::ErrorHandling(ErrorType::Result), 0.6);
        weights.insert(TokenClass::ErrorHandling(ErrorType::Option), 0.5);
        weights.insert(TokenClass::ErrorHandling(ErrorType::Unwrap), 0.8);
        weights.insert(TokenClass::ErrorHandling(ErrorType::Expect), 0.8);
        weights.insert(TokenClass::ErrorHandling(ErrorType::QuestionMark), 0.4);
        weights.insert(TokenClass::ErrorHandling(ErrorType::MapErr), 0.6);
        weights.insert(TokenClass::ErrorHandling(ErrorType::AndThen), 0.5);
        weights.insert(TokenClass::ErrorHandling(ErrorType::OrElse), 0.5);

        // Collection operations - moderate weight
        weights.insert(TokenClass::Collection(CollectionOp::Iteration), 0.3);
        weights.insert(TokenClass::Collection(CollectionOp::Mapping), 0.5);
        weights.insert(TokenClass::Collection(CollectionOp::Filtering), 0.5);
        weights.insert(TokenClass::Collection(CollectionOp::Aggregation), 0.7);
        weights.insert(TokenClass::Collection(CollectionOp::Access), 0.4);
        weights.insert(TokenClass::Collection(CollectionOp::Mutation), 0.6);

        // Literals - very low weight
        weights.insert(TokenClass::Literal(LiteralCategory::Numeric), 0.1);
        weights.insert(TokenClass::Literal(LiteralCategory::String), 0.2);
        weights.insert(TokenClass::Literal(LiteralCategory::Boolean), 0.1);
        weights.insert(TokenClass::Literal(LiteralCategory::Char), 0.1);
        weights.insert(TokenClass::Literal(LiteralCategory::Null), 0.1);

        Self {
            enabled: false, // Disabled by default for backward compatibility
            weights,
            cache_size: 10000,
        }
    }
}

/// Main token classifier
#[derive(Debug)]
pub struct TokenClassifier {
    config: ClassificationConfig,
    cache: HashMap<(String, bool, bool), TokenClass>,
}

impl TokenClassifier {
    pub fn new(config: ClassificationConfig) -> Self {
        Self {
            config,
            cache: HashMap::new(),
        }
    }

    pub fn classify(&mut self, token: &str, context: &TokenContext) -> TokenClass {
        if !self.config.enabled {
            return TokenClass::Unknown(token.to_string());
        }

        // Check cache first
        let cache_key = (
            token.to_string(),
            context.is_method_call,
            context.is_field_access,
        );
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        // Perform classification
        let class = self.classify_internal(token, context);

        // Update cache if not at capacity
        if self.cache.len() < self.config.cache_size {
            self.cache.insert(cache_key, class.clone());
        }

        class
    }

    fn classify_internal(&self, token: &str, context: &TokenContext) -> TokenClass {
        // Control flow keywords
        if matches!(token, "if" | "else" | "elif") {
            return TokenClass::ControlFlow(FlowType::If);
        }
        if token == "match" {
            return TokenClass::ControlFlow(FlowType::Match);
        }
        if token == "loop" {
            return TokenClass::ControlFlow(FlowType::Loop);
        }
        if token == "while" {
            return TokenClass::ControlFlow(FlowType::While);
        }
        if token == "for" {
            return TokenClass::ControlFlow(FlowType::For);
        }
        if token == "return" {
            return TokenClass::ControlFlow(FlowType::Return);
        }
        if token == "break" {
            return TokenClass::ControlFlow(FlowType::Break);
        }
        if token == "continue" {
            return TokenClass::ControlFlow(FlowType::Continue);
        }

        // Method calls
        if context.is_method_call {
            return self.classify_method_call(token, context);
        }

        // Field access
        if context.is_field_access {
            return self.classify_field_access(token, context);
        }

        // Local variables
        if !context.is_external && token.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return self.classify_local_var(token);
        }

        // Literals
        if token.parse::<f64>().is_ok() {
            return TokenClass::Literal(LiteralCategory::Numeric);
        }
        if token == "true" || token == "false" {
            return TokenClass::Literal(LiteralCategory::Boolean);
        }
        if token.starts_with('"') && token.ends_with('"') {
            return TokenClass::Literal(LiteralCategory::String);
        }
        if token.starts_with('\'') && token.ends_with('\'') && token.len() == 3 {
            return TokenClass::Literal(LiteralCategory::Char);
        }
        if token == "null" || token == "None" || token == "nil" {
            return TokenClass::Literal(LiteralCategory::Null);
        }

        // Keywords
        if matches!(
            token,
            "fn" | "let"
                | "const"
                | "mut"
                | "pub"
                | "struct"
                | "enum"
                | "trait"
                | "impl"
                | "mod"
                | "use"
                | "async"
                | "await"
                | "self"
                | "Self"
        ) {
            return TokenClass::Keyword(token.to_string());
        }

        // Operators
        if token.chars().all(|c| "+-*/%=<>!&|^~?.".contains(c)) {
            return TokenClass::Operator(token.to_string());
        }

        TokenClass::Unknown(token.to_string())
    }

    fn classify_method_call(&self, token: &str, context: &TokenContext) -> TokenClass {
        let lower = token.to_lowercase();

        // Getters
        if lower.starts_with("get_") || lower.ends_with("_ref") || lower.starts_with("as_") {
            return TokenClass::MethodCall(CallType::Getter);
        }

        // Setters
        if lower.starts_with("set_") || lower.starts_with("with_") {
            return TokenClass::MethodCall(CallType::Setter);
        }

        // Validators
        if lower.starts_with("is_")
            || lower.starts_with("has_")
            || lower.starts_with("can_")
            || lower.starts_with("should_")
        {
            return TokenClass::MethodCall(CallType::Validator);
        }

        // Converters
        if lower.starts_with("to_")
            || lower.starts_with("into_")
            || lower.starts_with("from_")
            || lower == "parse"
        {
            return TokenClass::MethodCall(CallType::Converter);
        }

        // I/O operations
        if matches!(
            lower.as_str(),
            "read"
                | "write"
                | "send"
                | "receive"
                | "recv"
                | "flush"
                | "sync"
                | "open"
                | "close"
                | "connect"
        ) {
            return TokenClass::MethodCall(CallType::IO);
        }

        // Error handling
        if matches!(
            lower.as_str(),
            "unwrap" | "expect" | "map_err" | "ok" | "err" | "and_then" | "or_else" | "unwrap_or"
        ) {
            return TokenClass::MethodCall(CallType::ErrorHandle);
        }

        // Collection operations
        if matches!(
            lower.as_str(),
            "push"
                | "pop"
                | "insert"
                | "remove"
                | "clear"
                | "len"
                | "is_empty"
                | "contains"
                | "get"
                | "iter"
                | "map"
                | "filter"
                | "fold"
                | "collect"
                | "sort"
        ) {
            return TokenClass::MethodCall(CallType::Collection);
        }

        // External API if marked as external
        if context.is_external {
            return TokenClass::MethodCall(CallType::External);
        }

        TokenClass::MethodCall(CallType::Other)
    }

    fn classify_field_access(&self, _token: &str, context: &TokenContext) -> TokenClass {
        // Simple classification based on context
        // Could be enhanced with more sophisticated analysis
        if context.parent_node_type == NodeType::Pattern {
            TokenClass::FieldAccess(AccessType::Getter)
        } else {
            TokenClass::FieldAccess(AccessType::Getter)
        }
    }

    fn classify_local_var(&self, token: &str) -> TokenClass {
        let lower = token.to_lowercase();

        // Iterators
        if matches!(
            lower.as_str(),
            "i" | "j" | "k" | "n" | "idx" | "index" | "iter" | "it" | "cursor"
        ) {
            return TokenClass::LocalVar(VarType::Iterator);
        }

        // Counters
        if lower.contains("count") || lower.contains("num") || lower.contains("total") {
            return TokenClass::LocalVar(VarType::Counter);
        }

        // Temporary
        if matches!(
            lower.as_str(),
            "temp" | "tmp" | "result" | "res" | "ret" | "val"
        ) {
            return TokenClass::LocalVar(VarType::Temporary);
        }

        // Configuration
        if lower.contains("config")
            || lower.contains("setting")
            || lower.contains("option")
            || lower.contains("param")
        {
            return TokenClass::LocalVar(VarType::Configuration);
        }

        // Resources
        if lower.contains("file")
            || lower.contains("conn")
            || lower.contains("client")
            || lower.contains("socket")
            || lower.contains("stream")
            || lower.contains("handle")
        {
            return TokenClass::LocalVar(VarType::Resource);
        }

        // Data
        if lower.contains("data")
            || lower.contains("value")
            || lower.contains("item")
            || lower.contains("element")
            || lower.contains("node")
            || lower.contains("entry")
        {
            return TokenClass::LocalVar(VarType::Data);
        }

        TokenClass::LocalVar(VarType::Other)
    }

    pub fn get_weight(&self, class: &TokenClass) -> f64 {
        self.config.weights.get(class).copied().unwrap_or(0.5)
    }

    pub fn update_weights(&mut self, weights: HashMap<TokenClass, f64>) {
        self.config.weights = weights;
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

/// Result of token classification with metadata
#[derive(Debug, Clone)]
pub struct ClassifiedToken {
    pub class: TokenClass,
    pub raw_token: String,
    pub context: TokenContext,
    pub weight: f64,
}

impl ClassifiedToken {
    pub fn new(class: TokenClass, raw_token: String, context: TokenContext, weight: f64) -> Self {
        Self {
            class,
            raw_token,
            context,
            weight,
        }
    }
}
