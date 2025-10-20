# God Object Detection

God objects (also called "god classes" or "god modules") are classes or modules with too many responsibilities. Debtmap detects and scores god objects to help identify architectural issues requiring refactoring.

## Overview

A god object violates the Single Responsibility Principle by taking on too many responsibilities. They:

- Are difficult to understand and maintain
- Have high coupling with many other modules
- Become bottlenecks for changes
- Are hard to test effectively

Debtmap identifies god objects using multiple criteria and provides a 0-100% god object score.

## Detection Criteria

### Method Count

Number of methods/functions in a class or module.

**Thresholds:**
- < 10 methods: Acceptable
- 10-20 methods: Moderate
- 20-30 methods: High (god object warning)
- 30+ methods: Very high (definite god object)

**Default configuration:**
```toml
[god_object]
max_methods = 20
```

### Field Count

Number of fields/attributes in a class or module state.

**Thresholds:**
- < 5 fields: Acceptable
- 5-15 fields: Moderate
- 15-25 fields: High (god object warning)
- 25+ fields: Very high (definite god object)

**Default configuration:**
```toml
[god_object]
max_fields = 15
```

### Responsibility Count

Number of distinct responsibilities (inferred from method names and dependencies).

**How it's calculated:**
- Group methods by semantic similarity
- Analyze dependency patterns
- Count distinct responsibility clusters

**Thresholds:**
- 1-2 responsibilities: Single purpose (good)
- 3-5 responsibilities: Multiple purposes (acceptable)
- 5-10 responsibilities: Too many purposes (god object)
- 10+ responsibilities: Extreme god object

**Default configuration:**
```toml
[god_object]
max_responsibilities = 5
```

### Lines of Code

Total lines of code in the class/module.

**Thresholds:**
- < 200 LOC: Small
- 200-500 LOC: Medium
- 500-1000 LOC: Large (consider splitting)
- 1000+ LOC: Very large (god object indicator)

## God Object Scoring

Debtmap calculates a god object score (0-100%):

```
God Object Score = weighted_sum(
    method_score × 0.35,
    field_score × 0.25,
    responsibility_score × 0.30,
    loc_score × 0.10
)
```

**Method score:**
```
method_score = min(100, (method_count / max_methods) × 100)
```

**Field score:**
```
field_score = min(100, (field_count / max_fields) × 100)
```

**Responsibility score:**
```
responsibility_score = min(100, (responsibility_count / max_responsibilities) × 100)
```

**LOC score:**
```
loc_score = min(100, (total_loc / 1000) × 100)
```

**Example:**
```
Class: UserManager
Methods: 45
Fields: 20
Responsibilities: 8 (auth, profile, permissions, notifications, logging, caching, validation, export)
LOC: 1200

method_score = min(100, (45 / 20) × 100) = 100
field_score = min(100, (20 / 15) × 100) = 100
responsibility_score = min(100, (8 / 5) × 100) = 100
loc_score = min(100, (1200 / 1000) × 100) = 100

God Object Score = (100 × 0.35) + (100 × 0.25) + (100 × 0.30) + (100 × 0.10)
                  = 35 + 25 + 30 + 10
                  = 100%

Assessment: DEFINITE GOD OBJECT
```

**Score interpretation:**
- 0-25%: Not a god object
- 25-50%: Potential god object (review)
- 50-75%: Likely god object (should refactor)
- 75-100%: Definite god object (refactor immediately)

## File-Level Aggregation

God objects are often identified at the file level:

```
File Score = Size × Complexity × Coverage × Density × GodObject × FunctionScores
```

**God object multiplier:**
```
god_object_multiplier = 2.0 + god_object_score
```

This significantly boosts file score when a god object is detected.

## Examples

### Example 1: Non-God Object

```rust
// src/user_repository.rs - 150 LOC
struct UserRepository {
    db: Database,
    cache: Cache,
}

impl UserRepository {
    fn find_by_id(&self, id: UserId) -> Result<User>;
    fn find_by_email(&self, email: &str) -> Result<User>;
    fn save(&mut self, user: &User) -> Result<()>;
    fn delete(&mut self, id: UserId) -> Result<()>;
}
```

**Analysis:**
- Methods: 4
- Fields: 2
- Responsibilities: 1 (data access)
- LOC: 150

**God Object Score:** 15% (not a god object)

### Example 2: God Object

```rust
// src/user_manager.rs - 1500 LOC
struct UserManager {
    db: Database,
    cache: Cache,
    mailer: Mailer,
    logger: Logger,
    session_store: SessionStore,
    permission_checker: PermissionChecker,
    validator: Validator,
    crypto: CryptoService,
    analytics: Analytics,
}

impl UserManager {
    // Authentication (8 methods)
    fn login(&mut self, credentials: Credentials) -> Result<Session>;
    fn logout(&mut self, session_id: &str) -> Result<()>;
    fn refresh_token(&mut self, token: &str) -> Result<Session>;
    // ... 5 more auth methods

    // Profile management (7 methods)
    fn get_profile(&self, user_id: UserId) -> Result<Profile>;
    fn update_profile(&mut self, profile: Profile) -> Result<()>;
    // ... 5 more profile methods

    // Permissions (6 methods)
    fn check_permission(&self, user_id: UserId, resource: &str) -> bool;
    fn grant_permission(&mut self, user_id: UserId, permission: Permission);
    // ... 4 more permission methods

    // Notifications (5 methods)
    fn send_notification(&self, user_id: UserId, notification: Notification);
    // ... 4 more notification methods

    // Logging and audit (4 methods)
    fn log_action(&self, action: Action);
    // ... 3 more logging methods

    // Data export (3 methods)
    fn export_user_data(&self, user_id: UserId) -> Result<Export>;
    // ... 2 more export methods

    // Cache management (4 methods)
    fn invalidate_cache(&mut self, user_id: UserId);
    // ... 3 more cache methods

    // Validation (6 methods)
    fn validate_email(&self, email: &str) -> Result<()>;
    // ... 5 more validation methods

    // Total: 43 methods, 9 fields, 8 responsibilities, 1500 LOC
}
```

**Analysis:**
- Methods: 43
- Fields: 9
- Responsibilities: 8 (auth, profile, permissions, notifications, logging, caching, validation, export)
- LOC: 1500

**God Object Score:** 95% (definite god object)

## Refactoring Recommendations

When debtmap detects a god object, it provides refactoring recommendations:

### Split by Responsibility

```rust
// Before: UserManager (god object)
struct UserManager { ... }

// After: Split into focused modules
struct AuthService { ... }
struct ProfileService { ... }
struct PermissionService { ... }
struct NotificationService { ... }
```

### Extract Common Functionality

```rust
// Extract shared dependencies
struct ServiceContext {
    db: Database,
    cache: Cache,
    logger: Logger,
}

// Each service gets a reference
struct AuthService<'a> {
    context: &'a ServiceContext,
}
```

### Use Composition

```rust
// Compose services instead of inheriting
struct UserFacade {
    auth: AuthService,
    profile: ProfileService,
    permissions: PermissionService,
}

impl UserFacade {
    fn login(&mut self, credentials: Credentials) -> Result<Session> {
        self.auth.login(credentials)
    }
}
```

## Configuration

Configure god object detection in `.debtmap.toml`:

```toml
[god_object]
# Enable god object detection (default: true)
enabled = true

# Maximum methods before flagging (default: 20)
max_methods = 20

# Maximum fields before flagging (default: 15)
max_fields = 15

# Maximum responsibilities before flagging (default: 5)
max_responsibilities = 5
```

### Tuning for Your Project

**Strict mode (smaller modules):**
```toml
[god_object]
max_methods = 15
max_fields = 10
max_responsibilities = 3
```

**Lenient mode (larger modules acceptable):**
```toml
[god_object]
max_methods = 30
max_fields = 20
max_responsibilities = 7
```

### Disable God Object Detection

```bash
debtmap analyze . --no-god-object
```

## Viewing God Objects

### In Terminal Output

```bash
debtmap analyze . --filter Architecture
```

Output:
```
#1 SCORE: 9.2 [CRITICAL] GOD_OBJECT
├─ FILE: ./src/user_manager.rs (1500 LOC)
├─ GOD_SCORE: 95% (methods: 43, fields: 9, resp: 8)
├─ ACTION: Split into 8 focused modules
└─ IMPACT: -8.5 complexity reduction
```

### In JSON Output

```bash
debtmap analyze . --format json --output report.json
```

```json
{
  "items": [
    {
      "type": "File",
      "location": {
        "file": "src/user_manager.rs",
        "line": 1
      },
      "debt_type": "GodObject",
      "score": 9.2,
      "god_object_score": 95,
      "god_object_metrics": {
        "method_count": 43,
        "field_count": 9,
        "responsibility_count": 8,
        "loc": 1500
      }
    }
  ]
}
```

## Best Practices

1. **Address god objects before feature work** - They create maintenance bottlenecks
2. **Split by responsibility** - Each module should have one clear purpose
3. **Use composition** - Combine services instead of inheriting
4. **Monitor over time** - Track god object scores with `compare` command
5. **Set appropriate thresholds** - Tune based on your project's architecture

## See Also

- [Tiered Prioritization](tiered-prioritization.md) - God objects are Tier 1 (critical architecture)
- [File-Level Scoring](scoring-strategies.md) - How god objects affect file scores
- [Configuration](configuration.md) - Complete configuration reference
- [Troubleshooting](troubleshooting.md) - General troubleshooting guide
