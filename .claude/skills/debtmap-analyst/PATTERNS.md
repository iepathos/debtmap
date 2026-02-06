# Debtmap Refactoring Patterns

Specific patterns for addressing common technical debt types identified by debtmap.

## Complexity Hotspots

### Pattern: Extract Classification Logic

When debtmap identifies high cognitive complexity from multiple conditionals:

```rust
// Before: Complexity 12+
fn analyze_token(token: &Token) -> TokenInfo {
    let kind = if token.text.starts_with("fn ") {
        TokenKind::Function
    } else if token.text.starts_with("struct ") {
        TokenKind::Struct
    } else if token.text.starts_with("enum ") {
        TokenKind::Enum
    } else if token.text.starts_with("impl ") {
        TokenKind::Impl
    } else if token.text.starts_with("//") || token.text.starts_with("/*") {
        TokenKind::Comment
    } else {
        TokenKind::Other
    };
    // More processing...
}

// After: Extract pure classifier
fn classify_token(text: &str) -> TokenKind {
    match () {
        _ if text.starts_with("fn ") => TokenKind::Function,
        _ if text.starts_with("struct ") => TokenKind::Struct,
        _ if text.starts_with("enum ") => TokenKind::Enum,
        _ if text.starts_with("impl ") => TokenKind::Impl,
        _ if text.starts_with("//") || text.starts_with("/*") => TokenKind::Comment,
        _ => TokenKind::Other,
    }
}

fn analyze_token(token: &Token) -> TokenInfo {
    let kind = classify_token(&token.text);
    // More processing...
}
```

### Pattern: Extract Nested Loop Logic

When debtmap flags nested loops increasing complexity:

```rust
// Before: Nested loops, complexity 8+
fn find_matches(items: &[Item], patterns: &[Pattern]) -> Vec<Match> {
    let mut matches = Vec::new();
    for item in items {
        for pattern in patterns {
            if pattern.matches(&item.name) && item.active {
                matches.push(Match {
                    item_id: item.id,
                    pattern_id: pattern.id,
                    score: calculate_score(item, pattern),
                });
            }
        }
    }
    matches
}

// After: Iterator-based with helper
fn check_match(item: &Item, pattern: &Pattern) -> Option<Match> {
    (pattern.matches(&item.name) && item.active).then(|| Match {
        item_id: item.id,
        pattern_id: pattern.id,
        score: calculate_score(item, pattern),
    })
}

fn find_matches(items: &[Item], patterns: &[Pattern]) -> Vec<Match> {
    items
        .iter()
        .flat_map(|item| patterns.iter().filter_map(|pat| check_match(item, pat)))
        .collect()
}
```

## God Objects

### Pattern: Split by Domain

When debtmap identifies a god object (file with many responsibilities):

```rust
// Before: utils.rs with 50 functions
// - String manipulation (10 functions)
// - Date handling (8 functions)
// - File operations (12 functions)
// - Formatting (20 functions)

// After: Split into focused modules
mod string_utils {
    pub fn normalize(s: &str) -> String { ... }
    pub fn truncate(s: &str, max: usize) -> String { ... }
    // ...
}

mod date_utils {
    pub fn parse_date(s: &str) -> Result<Date> { ... }
    pub fn format_date(d: &Date) -> String { ... }
    // ...
}

mod file_ops {
    pub fn read_lines(path: &Path) -> Result<Vec<String>> { ... }
    pub fn write_atomic(path: &Path, content: &str) -> Result<()> { ... }
    // ...
}

mod formatters {
    pub fn format_table(data: &[Row]) -> String { ... }
    pub fn format_json(value: &Value) -> String { ... }
    // ...
}
```

### Pattern: Extract Service Modules

For god objects that mix data and behavior:

```rust
// Before: analyzer.rs with 2000+ lines
struct Analyzer {
    config: Config,
    cache: HashMap<PathBuf, Analysis>,
    // 20 more fields
}

impl Analyzer {
    fn parse(&self, content: &str) -> Ast { ... }
    fn analyze(&self, ast: &Ast) -> Metrics { ... }
    fn format(&self, metrics: &Metrics) -> String { ... }
    fn cache_result(&mut self, path: &Path, result: &Analysis) { ... }
    // 50 more methods
}

// After: Split into focused modules
// parser.rs - Pure parsing logic
pub fn parse(content: &str) -> Result<Ast> { ... }

// analyzer.rs - Pure analysis logic
pub fn analyze(ast: &Ast, config: &AnalysisConfig) -> Metrics { ... }

// formatter.rs - Pure formatting
pub fn format(metrics: &Metrics, format: OutputFormat) -> String { ... }

// cache.rs - Caching concerns
pub struct AnalysisCache { ... }

// orchestrator.rs - Thin coordination layer
pub struct AnalysisOrchestrator {
    config: Config,
    cache: AnalysisCache,
}

impl AnalysisOrchestrator {
    pub fn process(&mut self, path: &Path) -> Result<String> {
        if let Some(cached) = self.cache.get(path) {
            return Ok(format(&cached.metrics, self.config.format));
        }
        let content = fs::read_to_string(path)?;
        let ast = parse(&content)?;
        let metrics = analyze(&ast, &self.config.analysis);
        self.cache.insert(path, &metrics);
        Ok(format(&metrics, self.config.format))
    }
}
```

## Coverage Gaps

### Pattern: Extract Testable Logic from I/O

When debtmap flags low coverage on I/O-heavy functions:

```rust
// Before: Untested because it does I/O
fn generate_report(db: &Database, output_path: &Path) -> Result<()> {
    let users = db.fetch_all_users()?;

    let mut report = String::from("# User Report\n\n");
    for user in &users {
        let status = if user.active { "Active" } else { "Inactive" };
        let age_group = if user.age < 18 {
            "Minor"
        } else if user.age < 65 {
            "Adult"
        } else {
            "Senior"
        };
        report.push_str(&format!("- {} ({}): {}\n", user.name, age_group, status));
    }

    let summary = format!("\nTotal: {} users\n", users.len());
    report.push_str(&summary);

    fs::write(output_path, &report)?;
    Ok(())
}

// After: Pure logic extracted and testable
fn classify_age(age: u8) -> &'static str {
    match age {
        0..=17 => "Minor",
        18..=64 => "Adult",
        _ => "Senior",
    }
}

fn format_user_line(user: &User) -> String {
    let status = if user.active { "Active" } else { "Inactive" };
    format!("- {} ({}): {}\n", user.name, classify_age(user.age), status)
}

fn build_report(users: &[User]) -> String {
    let mut report = String::from("# User Report\n\n");
    for user in users {
        report.push_str(&format_user_line(user));
    }
    report.push_str(&format!("\nTotal: {} users\n", users.len()));
    report
}

// Thin I/O wrapper - doesn't need testing
fn generate_report(db: &Database, output_path: &Path) -> Result<()> {
    let users = db.fetch_all_users()?;
    let report = build_report(&users);
    fs::write(output_path, &report)?;
    Ok(())
}

// Now testable!
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_age() {
        assert_eq!(classify_age(17), "Minor");
        assert_eq!(classify_age(18), "Adult");
        assert_eq!(classify_age(65), "Senior");
    }

    #[test]
    fn test_build_report() {
        let users = vec![
            User { name: "Alice".into(), age: 30, active: true },
            User { name: "Bob".into(), age: 70, active: false },
        ];
        let report = build_report(&users);
        assert!(report.contains("Alice (Adult): Active"));
        assert!(report.contains("Bob (Senior): Inactive"));
        assert!(report.contains("Total: 2 users"));
    }
}
```

## Error Handling Debt

### Pattern: Context-Rich Errors

When debtmap identifies error swallowing or poor error context:

```rust
// Before: Lost context
fn process_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)?;  // "No such file"
    let config: Config = toml::from_str(&content)?;  // "expected `=`"
    validate_config(&config)?;  // "invalid value"
    Ok(config)
}

// After: Rich context chain
fn process_config(path: &Path) -> Result<Config> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {:?}", path))?;

    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Invalid TOML in {:?}", path))?;

    validate_config(&config)
        .with_context(|| format!("Config validation failed for {:?}", path))?;

    Ok(config)
}

// Error output now shows:
// Config validation failed for "./config.toml"
// Caused by: threshold must be positive, got -5
```

### Pattern: Validation Accumulation

When debtmap identifies fail-fast validation that should accumulate:

```rust
// Before: Stops at first error
fn validate_user(input: &UserInput) -> Result<ValidUser> {
    let email = validate_email(&input.email)?;     // Stops here
    let password = validate_password(&input.password)?;  // Never reached
    let age = validate_age(input.age)?;            // Never reached
    Ok(ValidUser { email, password, age })
}

// After: Accumulates all errors
use stillwater::Validation;

fn validate_user(input: &UserInput) -> Validation<ValidUser, Vec<ValidationError>> {
    Validation::all((
        validate_email(&input.email),
        validate_password(&input.password),
        validate_age(input.age),
    ))
    .map(|(email, password, age)| ValidUser { email, password, age })
}
// Returns ALL errors: [EmailInvalid, PasswordTooShort, AgeTooLow]
```

## Performance Debt

### Pattern: Eliminate Redundant Allocations

When debtmap flags allocation-heavy code:

```rust
// Before: Multiple allocations
fn process_names(names: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    for name in names {
        let lower = name.to_lowercase();  // Allocation
        let trimmed = lower.trim().to_string();  // Allocation
        if !trimmed.is_empty() {
            result.push(trimmed);
        }
    }
    result
}

// After: Minimized allocations
fn process_names(names: &[String]) -> Vec<String> {
    names
        .iter()
        .map(|name| name.trim())
        .filter(|name| !name.is_empty())
        .map(|name| name.to_lowercase())  // Single allocation per item
        .collect()
}

// Even better with Cow for conditional allocation
use std::borrow::Cow;

fn process_name(name: &str) -> Option<Cow<'_, str>> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.chars().all(|c| c.is_lowercase()) {
        Some(Cow::Borrowed(trimmed))  // No allocation
    } else {
        Some(Cow::Owned(trimmed.to_lowercase()))  // Allocation only when needed
    }
}
```

### Pattern: Lazy Iteration

When debtmap flags eager collection where lazy would work:

```rust
// Before: Collects immediately
fn find_first_match(items: &[Item], pattern: &str) -> Option<&Item> {
    let matches: Vec<_> = items
        .iter()
        .filter(|i| i.name.contains(pattern))
        .collect();  // Allocates even if we only need first
    matches.first().copied()
}

// After: Lazy iteration
fn find_first_match(items: &[Item], pattern: &str) -> Option<&Item> {
    items.iter().find(|i| i.name.contains(pattern))
}
```

## Code Duplication

### Pattern: Extract Shared Logic

When debtmap identifies duplicated code:

```rust
// Before: Duplicated validation logic
fn validate_user_email(email: &str) -> Result<(), Error> {
    if email.is_empty() {
        return Err(Error::Empty("email"));
    }
    if !email.contains('@') {
        return Err(Error::Invalid("email", "must contain @"));
    }
    Ok(())
}

fn validate_admin_email(email: &str) -> Result<(), Error> {
    if email.is_empty() {
        return Err(Error::Empty("admin email"));
    }
    if !email.contains('@') {
        return Err(Error::Invalid("admin email", "must contain @"));
    }
    if !email.ends_with("@company.com") {
        return Err(Error::Invalid("admin email", "must be company email"));
    }
    Ok(())
}

// After: Composed validation
fn validate_email_format(email: &str, field: &str) -> Result<(), Error> {
    if email.is_empty() {
        return Err(Error::Empty(field));
    }
    if !email.contains('@') {
        return Err(Error::Invalid(field, "must contain @"));
    }
    Ok(())
}

fn validate_company_domain(email: &str, field: &str) -> Result<(), Error> {
    if !email.ends_with("@company.com") {
        return Err(Error::Invalid(field, "must be company email"));
    }
    Ok(())
}

fn validate_user_email(email: &str) -> Result<(), Error> {
    validate_email_format(email, "email")
}

fn validate_admin_email(email: &str) -> Result<(), Error> {
    validate_email_format(email, "admin email")?;
    validate_company_domain(email, "admin email")
}
```

## Testing Debt

### Pattern: Parameterized Tests

When debtmap flags untested edge cases:

```rust
// Before: Missing edge cases
#[test]
fn test_calculate_discount() {
    assert_eq!(calculate_discount(100.0, false), 95.0);
}

// After: Comprehensive coverage with parameterization
#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    #[rstest]
    #[case(100.0, false, 95.0)]   // Regular customer
    #[case(100.0, true, 85.0)]    // Premium customer
    #[case(0.0, false, 0.0)]      // Zero amount
    #[case(0.0, true, 0.0)]       // Zero amount premium
    #[case(1000.0, false, 950.0)] // Large amount
    fn test_calculate_discount(
        #[case] amount: f64,
        #[case] is_premium: bool,
        #[case] expected: f64,
    ) {
        assert_eq!(calculate_discount(amount, is_premium), expected);
    }
}
```

### Pattern: Test Pure Logic, Mock I/O

When debtmap flags untested business logic mixed with I/O:

```rust
// The key insight: Test the PURE functions, not the I/O wrappers

// Pure logic - MUST test
fn calculate_order_total(items: &[OrderItem], discount: f64) -> f64 { ... }
fn validate_order(order: &Order) -> Result<(), ValidationError> { ... }
fn format_receipt(order: &Order, total: f64) -> String { ... }

// I/O wrapper - optional to test
async fn process_order(order_id: OrderId, db: &Database) -> Result<Receipt> {
    let order = db.fetch_order(order_id).await?;
    validate_order(&order)?;  // Tested separately
    let total = calculate_order_total(&order.items, order.discount);  // Tested separately
    let receipt_text = format_receipt(&order, total);  // Tested separately
    db.save_receipt(order_id, &receipt_text).await?;
    Ok(Receipt { order_id, total, text: receipt_text })
}
```
