# Specification 40: JavaScript/TypeScript Detector Implementation Details

## Problem Statement

JavaScript and TypeScript analysis currently uses tree-sitter for basic parsing but lacks comprehensive debt detection. We need to leverage tree-sitter's CST (Concrete Syntax Tree) to implement the same detector categories available for Rust.

## Technical Design

### Tree-Sitter Query Pattern System

```rust
use tree_sitter::{Query, QueryCursor, Node};

pub struct JavaScriptDetectorVisitor {
    source_content: String,
    path: PathBuf,
    language: tree_sitter::Language,
    performance_patterns: Vec<PerformanceAntiPattern>,
    organization_patterns: Vec<OrganizationAntiPattern>,
    security_vulnerabilities: Vec<SecurityVulnerability>,
    resource_issues: Vec<ResourceManagementIssue>,
    testing_issues: Vec<TestingAntiPattern>,
}

impl JavaScriptDetectorVisitor {
    pub fn new_with_queries() -> Self {
        // Initialize tree-sitter queries for pattern matching
        let nested_loop_query = Query::new(
            language,
            r#"
            (for_statement
              body: (block_statement
                (for_statement) @inner_loop
              )
            ) @outer_loop
            "#
        ).unwrap();
        
        // More queries...
    }
}
```

### Performance Pattern Detection

#### Async/Await Anti-patterns
```rust
fn detect_sequential_awaits(&mut self, node: Node, source: &str) {
    let query = Query::new(
        self.language,
        r#"
        (block_statement
          (expression_statement
            (await_expression) @await1
          )
          (expression_statement
            (await_expression) @await2
          )
        )
        "#
    ).unwrap();
    
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, node, source.as_bytes());
    
    for match_ in matches {
        // Check if these awaits could be parallelized
        let await1 = match_.captures[0].node;
        let await2 = match_.captures[1].node;
        
        if self.are_independent_operations(await1, await2) {
            self.performance_patterns.push(
                PerformanceAntiPattern::InefficientIO {
                    io_pattern: IOPattern::SequentialAsync,
                    batching_opportunity: true,
                    async_opportunity: true,
                    location: self.extract_location(await1),
                }
            );
        }
    }
}

fn detect_missing_promise_all(&mut self, node: Node, source: &str) {
    // Detect patterns like:
    // const a = await fetchA();
    // const b = await fetchB();
    // const c = await fetchC();
    // Should be: const [a, b, c] = await Promise.all([...])
    
    let query = Query::new(
        self.language,
        r#"
        (variable_declaration
          (variable_declarator
            init: (await_expression
              (call_expression) @async_call
            )
          )
        ) @declaration
        "#
    ).unwrap();
    
    // Count consecutive await declarations
    let mut consecutive_awaits = Vec::new();
    // ... detection logic
}
```

#### DOM Performance Issues
```rust
fn detect_layout_thrashing(&mut self, node: Node, source: &str) {
    // Detect read-write-read-write patterns on DOM properties
    let query = Query::new(
        self.language,
        r#"
        (block_statement
          (expression_statement
            (member_expression
              property: (property_identifier) @prop
            )
          )*
        )
        "#
    ).unwrap();
    
    let mut dom_accesses = Vec::new();
    let mut cursor = QueryCursor::new();
    
    for match_ in cursor.matches(&query, node, source.as_bytes()) {
        let prop_name = self.get_node_text(match_.captures[0].node, source);
        if self.is_layout_property(&prop_name) {
            dom_accesses.push((prop_name, match_.captures[0].node));
        }
    }
    
    // Check for interleaved reads and writes
    if self.has_layout_thrashing(&dom_accesses) {
        self.performance_patterns.push(
            PerformanceAntiPattern::DOMThrashing {
                operations: dom_accesses.len(),
                location: self.extract_location(node),
                suggestion: "Batch DOM reads and writes separately".to_string(),
            }
        );
    }
}

fn is_layout_property(&self, prop: &str) -> bool {
    matches!(prop, 
        "offsetWidth" | "offsetHeight" | "offsetTop" | "offsetLeft" |
        "scrollWidth" | "scrollHeight" | "scrollTop" | "scrollLeft" |
        "clientWidth" | "clientHeight" | "clientTop" | "clientLeft" |
        "getComputedStyle" | "getBoundingClientRect"
    )
}
```

### Organization Pattern Detection

#### Module Circular Dependencies
```rust
fn detect_circular_dependencies(&mut self, node: Node, source: &str) {
    // Build import/export graph
    let import_query = Query::new(
        self.language,
        r#"
        [
          (import_statement
            source: (string) @source
          )
          (call_expression
            function: (identifier) @func (#eq? @func "require")
            arguments: (arguments (string) @source)
          )
        ]
        "#
    ).unwrap();
    
    let mut imports = HashMap::new();
    let mut cursor = QueryCursor::new();
    
    for match_ in cursor.matches(&import_query, node, source.as_bytes()) {
        let source_path = self.get_node_text(match_.captures[0].node, source);
        imports.entry(self.path.clone())
            .or_insert_with(Vec::new)
            .push(source_path);
    }
    
    // Check for cycles using DFS
    if let Some(cycle) = self.find_import_cycle(&imports) {
        self.organization_patterns.push(
            OrganizationAntiPattern::CircularDependency {
                modules: cycle,
                location: self.extract_location(node),
            }
        );
    }
}
```

#### TypeScript `any` Overuse
```rust
fn detect_any_type_overuse(&mut self, node: Node, source: &str) {
    let query = Query::new(
        self.language,
        r#"
        (type_annotation
          (any_type) @any
        )
        "#
    ).unwrap();
    
    let mut cursor = QueryCursor::new();
    let any_count = cursor.matches(&query, node, source.as_bytes()).count();
    
    if any_count > 5 {  // Threshold
        self.organization_patterns.push(
            OrganizationAntiPattern::TypeSafetyViolation {
                violation_type: "Excessive use of 'any' type".to_string(),
                count: any_count,
                location: self.extract_location(node),
                suggestion: "Use specific types or unknown".to_string(),
            }
        );
    }
}
```

### Security Pattern Detection

#### XSS Vulnerability Detection
```rust
fn detect_xss_vulnerabilities(&mut self, node: Node, source: &str) {
    // Detect innerHTML with user input
    let query = Query::new(
        self.language,
        r#"
        (assignment_expression
          left: (member_expression
            property: (property_identifier) @prop (#eq? @prop "innerHTML")
          )
          right: (_) @value
        )
        "#
    ).unwrap();
    
    let mut cursor = QueryCursor::new();
    for match_ in cursor.matches(&query, node, source.as_bytes()) {
        let value_node = match_.captures[1].node;
        if self.contains_user_input(value_node, source) {
            self.security_vulnerabilities.push(
                SecurityVulnerability::XSS {
                    sink: "innerHTML".to_string(),
                    user_input_source: self.trace_input_source(value_node),
                    severity: Severity::Critical,
                    location: self.extract_location(value_node),
                }
            );
        }
    }
}

fn detect_eval_usage(&mut self, node: Node, source: &str) {
    let query = Query::new(
        self.language,
        r#"
        (call_expression
          function: [
            (identifier) @func (#match? @func "^(eval|Function)$")
            (member_expression
              object: (identifier) @obj (#eq? @obj "window")
              property: (property_identifier) @prop (#eq? @prop "eval")
            )
          ]
          arguments: (_) @args
        )
        "#
    ).unwrap();
    
    let mut cursor = QueryCursor::new();
    for match_ in cursor.matches(&query, node, source.as_bytes()) {
        self.security_vulnerabilities.push(
            SecurityVulnerability::CodeInjection {
                vulnerability_type: CodeInjectionType::EvalUsage,
                severity: Severity::High,
                location: self.extract_location(match_.captures[0].node),
            }
        );
    }
}
```

### Resource Management Detection

#### Memory Leak Detection
```rust
fn detect_event_listener_leaks(&mut self, node: Node, source: &str) {
    // Find addEventListener without corresponding removeEventListener
    let add_query = Query::new(
        self.language,
        r#"
        (call_expression
          function: (member_expression
            property: (property_identifier) @method (#eq? @method "addEventListener")
          )
          arguments: (arguments
            (string) @event
            (_) @handler
          )
        ) @add_call
        "#
    ).unwrap();
    
    let remove_query = Query::new(
        self.language,
        r#"
        (call_expression
          function: (member_expression
            property: (property_identifier) @method (#eq? @method "removeEventListener")
          )
          arguments: (arguments
            (string) @event
            (_) @handler
          )
        ) @remove_call
        "#
    ).unwrap();
    
    // Track added vs removed listeners
    let mut added_listeners = HashMap::new();
    let mut removed_listeners = HashSet::new();
    
    // ... matching logic
    
    for (event, handler) in added_listeners {
        if !removed_listeners.contains(&(event.clone(), handler.clone())) {
            self.resource_issues.push(
                ResourceManagementIssue::HandleLeak {
                    handle_type: HandleType::EventListener,
                    leak_location: self.extract_location(node),
                    proper_cleanup: format!("Call removeEventListener for '{}'", event),
                }
            );
        }
    }
}

fn detect_timer_leaks(&mut self, node: Node, source: &str) {
    // Detect setInterval/setTimeout without clearInterval/clearTimeout
    let timer_query = Query::new(
        self.language,
        r#"
        (call_expression
          function: (identifier) @func (#match? @func "^(setTimeout|setInterval)$")
        ) @timer_call
        "#
    ).unwrap();
    
    // Check if timer IDs are stored and cleared
    // ... implementation
}
```

### Testing Pattern Detection

#### React Testing Issues
```rust
fn detect_react_test_issues(&mut self, node: Node, source: &str) {
    // Detect missing cleanup in React tests
    let query = Query::new(
        self.language,
        r#"
        (call_expression
          function: (identifier) @func (#match? @func "^(render|mount)$")
        ) @render_call
        "#
    ).unwrap();
    
    // Check if there's a corresponding cleanup/unmount
    let cleanup_query = Query::new(
        self.language,
        r#"
        (call_expression
          function: [
            (identifier) @func (#match? @func "^(cleanup|unmount)$")
            (member_expression
              property: (property_identifier) @prop (#eq? @prop "unmount")
            )
          ]
        )
        "#
    ).unwrap();
    
    // ... detection logic
}

fn detect_async_test_issues(&mut self, node: Node, source: &str) {
    // Detect tests without proper async handling
    let test_query = Query::new(
        self.language,
        r#"
        (call_expression
          function: (identifier) @func (#match? @func "^(test|it|describe)$")
          arguments: (arguments
            (string) @test_name
            (arrow_function
              async: false
              body: (block_statement) @body
            )
          )
        )
        "#
    ).unwrap();
    
    // Check if test body contains async operations without await
    // ... implementation
}
```

### Framework-Specific Detection

#### React-Specific Patterns
```rust
mod react_patterns {
    pub fn detect_missing_deps(&mut self, node: Node, source: &str) {
        // Detect missing dependencies in useEffect, useCallback, useMemo
        let query = Query::new(
            self.language,
            r#"
            (call_expression
              function: (identifier) @hook (#match? @hook "^use(Effect|Callback|Memo)$")
              arguments: (arguments
                (_) @callback
                (array) @deps
              )
            )
            "#
        ).unwrap();
        
        // Analyze callback for external references
        // Compare with dependency array
        // ... implementation
    }
    
    pub fn detect_state_mutations(&mut self, node: Node, source: &str) {
        // Detect direct state mutations
        let query = Query::new(
            self.language,
            r#"
            (assignment_expression
              left: (member_expression
                object: (member_expression
                  object: (this) @this
                  property: (property_identifier) @state (#eq? @state "state")
                )
              )
            )
            "#
        ).unwrap();
        
        // ... detection logic
    }
}
```

#### Vue-Specific Patterns
```rust
mod vue_patterns {
    pub fn detect_lifecycle_issues(&mut self, node: Node, source: &str) {
        // Detect improper lifecycle hook usage
        // ... implementation
    }
}
```

#### Angular-Specific Patterns
```rust
mod angular_patterns {
    pub fn detect_subscription_leaks(&mut self, node: Node, source: &str) {
        // Detect Observable subscriptions without unsubscribe
        let subscribe_query = Query::new(
            self.language,
            r#"
            (call_expression
              function: (member_expression
                property: (property_identifier) @method (#eq? @method "subscribe")
              )
            ) @subscription
            "#
        ).unwrap();
        
        // Check for corresponding unsubscribe in ngOnDestroy
        // ... implementation
    }
}
```

### Location Extraction for Tree-Sitter

```rust
impl JavaScriptDetectorVisitor {
    fn extract_location(&self, node: Node) -> SourceLocation {
        let start = node.start_position();
        let end = node.end_position();
        
        SourceLocation {
            line: start.row + 1,  // tree-sitter uses 0-based lines
            column: Some(start.column),
            end_line: Some(end.row + 1),
            end_column: Some(end.column),
            confidence: LocationConfidence::Exact,
        }
    }
    
    fn get_node_text<'a>(&self, node: Node, source: &'a str) -> &'a str {
        node.utf8_text(source.as_bytes()).unwrap_or("")
    }
}
```

## JavaScript/TypeScript-Specific Patterns

### Performance Anti-patterns
1. **Sequential Promise Resolution**: Not using `Promise.all()`
2. **Repeated DOM Queries**: Not caching DOM selections
3. **Layout Thrashing**: Interleaved DOM reads/writes
4. **Large Bundle Imports**: Importing entire libraries
5. **Synchronous XHR**: Using synchronous XMLHttpRequest

### Organization Anti-patterns
1. **Callback Hell**: Deeply nested callbacks
2. **Global Namespace Pollution**: Too many global variables
3. **Mixed Module Systems**: CommonJS and ES6 modules mixed
4. **Prototype Pollution Risk**: Unsafe object property access
5. **Complex Type Gymnastics**: Over-engineered TypeScript types

### Security Vulnerabilities
1. **XSS via innerHTML**: Direct HTML injection
2. **Prototype Pollution**: `__proto__` manipulation
3. **JSON Injection**: Unsafe JSON parsing
4. **CSRF Missing**: No CSRF token validation
5. **Insecure Random**: Using Math.random() for security

### Resource Management Issues
1. **Event Listener Leaks**: Not removing listeners
2. **Timer Leaks**: Uncancelled setInterval/setTimeout
3. **WebSocket Leaks**: Unclosed connections
4. **Worker Thread Leaks**: Not terminating workers
5. **Memory Retention**: Large closures keeping references

### Testing Anti-patterns
1. **Missing Test Cleanup**: React components not unmounted
2. **Timing-Dependent Tests**: Using setTimeout in tests
3. **Missing Mocks**: Testing with real network calls
4. **Snapshot Overuse**: Brittle snapshot tests
5. **Missing Error Boundaries**: No error handling in tests

## Integration Points

```rust
// In src/analyzers/javascript.rs and typescript.rs
impl Analyzer for JavaScriptAnalyzer {
    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::JavaScript(js_ast) | Ast::TypeScript(js_ast) => {
                let mut metrics = analyze_js_file(js_ast, self.complexity_threshold);
                
                // Add comprehensive detector analysis
                let mut detector = JavaScriptDetectorVisitor::new(
                    &js_ast.source,
                    &js_ast.path,
                    js_ast.tree,
                    js_ast.language
                );
                
                detector.visit_tree();
                
                // Convert detected patterns to debt items
                metrics.debt_items.extend(detector.to_debt_items());
                
                metrics
            }
            _ => // ...
        }
    }
}
```

## Performance Considerations

1. **Query Compilation**: Compile tree-sitter queries once, reuse
2. **Incremental Parsing**: Use tree-sitter's incremental parsing
3. **Parallel Analysis**: Process independent patterns in parallel
4. **Early Termination**: Stop on critical issues
5. **Memory Management**: Stream large files instead of loading fully

## Success Metrics

1. **Framework Coverage**: Support React, Vue, Angular, Node.js patterns
2. **ES6+ Support**: Full modern JavaScript syntax support
3. **TypeScript Support**: Type-aware analysis for TS files
4. **Performance**: < 100ms per file for average JS/TS file
5. **Accuracy**: Match ESLint/TSLint rule detection rates