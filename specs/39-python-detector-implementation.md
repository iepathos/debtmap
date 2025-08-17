# Specification 39: Python Detector Implementation Details

## Problem Statement

Python analysis in debtmap currently only supports basic complexity metrics, TODOs, and code smells. The rustpython-parser provides a full Python AST, but we're not leveraging it for comprehensive debt detection like we do for Rust.

## Technical Design

### AST Visitor Pattern for Python

```rust
use rustpython_parser::ast::{Stmt, Expr, ExprKind, StmtKind};

pub struct PythonDetectorVisitor {
    source_content: String,
    path: PathBuf,
    performance_patterns: Vec<PerformanceAntiPattern>,
    organization_patterns: Vec<OrganizationAntiPattern>,
    security_vulnerabilities: Vec<SecurityVulnerability>,
    resource_issues: Vec<ResourceManagementIssue>,
    testing_issues: Vec<TestingAntiPattern>,
}

impl PythonDetectorVisitor {
    pub fn visit_stmt(&mut self, stmt: &Stmt) {
        match &stmt.node {
            StmtKind::For { .. } => self.check_nested_loops(stmt),
            StmtKind::FunctionDef { .. } => self.check_function_patterns(stmt),
            StmtKind::ClassDef { .. } => self.check_class_patterns(stmt),
            StmtKind::With { .. } => self.check_resource_management(stmt),
            _ => {}
        }
    }
}
```

### Performance Pattern Detection

#### Nested Loop Detection
```rust
fn check_nested_loops(&mut self, stmt: &Stmt) {
    struct LoopNestingVisitor {
        nesting_level: u32,
        max_nesting: u32,
        loop_locations: Vec<Location>,
    }
    
    impl LoopNestingVisitor {
        fn visit_for(&mut self, stmt: &Stmt) {
            self.nesting_level += 1;
            self.max_nesting = self.max_nesting.max(self.nesting_level);
            if self.nesting_level >= 2 {
                self.loop_locations.push(stmt.location);
            }
            // Recursively visit body
            self.nesting_level -= 1;
        }
    }
}
```

#### String Concatenation in Loops
```rust
fn detect_string_concat_in_loop(&mut self, loop_stmt: &Stmt) {
    // Detect patterns like:
    // result = ""
    // for item in items:
    //     result += str(item)  # Anti-pattern
    
    if let StmtKind::For { body, .. } = &loop_stmt.node {
        for stmt in body {
            if let StmtKind::AugAssign { op: Operator::Add, target, .. } = &stmt.node {
                if self.is_string_type(target) {
                    self.performance_patterns.push(
                        PerformanceAntiPattern::StringProcessingAntiPattern {
                            pattern_type: StringAntiPattern::ConcatenationInLoop,
                            performance_impact: PerformanceImpact::High,
                            recommended_approach: "Use ''.join() instead".to_string(),
                            location: self.extract_location(stmt),
                        }
                    );
                }
            }
        }
    }
}
```

#### List Comprehension Complexity
```rust
fn analyze_comprehension_complexity(&mut self, expr: &Expr) {
    if let ExprKind::ListComp { generators, .. } = &expr.node {
        let nested_count = generators.len();
        if nested_count >= 3 {
            self.performance_patterns.push(
                PerformanceAntiPattern::NestedLoop {
                    nesting_level: nested_count as u32,
                    estimated_complexity: ComplexityClass::Exponential,
                    inner_operations: vec![LoopOperation::Computation],
                    can_parallelize: false,
                    location: self.extract_location(expr),
                }
            );
        }
    }
}
```

### Organization Pattern Detection

#### God Class Detection
```rust
fn check_god_class(&mut self, class_def: &Stmt) {
    if let StmtKind::ClassDef { name, body, .. } = &class_def.node {
        let method_count = body.iter().filter(|s| {
            matches!(s.node, StmtKind::FunctionDef { .. })
        }).count();
        
        let field_count = self.count_class_fields(body);
        
        if method_count > 20 || field_count > 15 {
            self.organization_patterns.push(
                OrganizationAntiPattern::GodObject {
                    type_name: name.to_string(),
                    method_count,
                    field_count,
                    suggested_split: self.suggest_class_split(body),
                    location: self.extract_location(class_def),
                }
            );
        }
    }
}
```

#### Magic Value Detection
```rust
fn detect_magic_values(&mut self, expr: &Expr) {
    match &expr.node {
        ExprKind::Constant { value, .. } => {
            if let Some(magic) = self.is_magic_value(value) {
                self.track_magic_value(magic, expr.location);
            }
        }
        _ => {}
    }
}

fn is_magic_value(&self, value: &Constant) -> Option<String> {
    match value {
        Constant::Int(n) if *n != 0 && *n != 1 && *n != -1 => {
            Some(n.to_string())
        }
        Constant::Float(f) if !f.is_nan() && *f != 0.0 && *f != 1.0 => {
            Some(f.to_string())
        }
        Constant::Str(s) if s.len() > 3 && !self.is_common_string(s) => {
            Some(s.clone())
        }
        _ => None
    }
}
```

### Security Pattern Detection

#### Unsafe Eval Detection
```rust
fn detect_unsafe_eval(&mut self, expr: &Expr) {
    if let ExprKind::Call { func, args, .. } = &expr.node {
        if let ExprKind::Name { id, .. } = &func.node {
            match id.as_str() {
                "eval" | "exec" | "compile" => {
                    if self.has_user_input(args) {
                        self.security_vulnerabilities.push(
                            SecurityVulnerability::CodeInjection {
                                vulnerability_type: CodeInjectionType::EvalUsage,
                                severity: Severity::Critical,
                                user_input_source: self.trace_input_source(args),
                                location: self.extract_location(expr),
                            }
                        );
                    }
                }
                _ => {}
            }
        }
    }
}
```

#### SQL Injection Detection
```rust
fn detect_sql_injection(&mut self, expr: &Expr) {
    if let ExprKind::Call { func, args, .. } = &expr.node {
        if self.is_sql_execute_call(func) {
            if let Some(query_arg) = args.first() {
                if self.has_string_formatting(query_arg) {
                    self.security_vulnerabilities.push(
                        SecurityVulnerability::SqlInjection {
                            query_construction: "String formatting".to_string(),
                            parameterization_missing: true,
                            severity: Severity::Critical,
                            location: self.extract_location(expr),
                        }
                    );
                }
            }
        }
    }
}

fn has_string_formatting(&self, expr: &Expr) -> bool {
    matches!(expr.node, 
        ExprKind::BinOp { op: Operator::Mod, .. } |  // % formatting
        ExprKind::JoinedStr { .. } |  // f-strings with variables
        ExprKind::Call { func, .. } if self.is_format_call(func)  // .format()
    )
}
```

### Resource Management Detection

#### Missing Context Manager
```rust
fn detect_resource_without_context_manager(&mut self, stmt: &Stmt) {
    if let StmtKind::Assign { targets, value, .. } = &stmt.node {
        if let ExprKind::Call { func, .. } = &value.node {
            if self.is_resource_acquisition(func) {
                // Check if this is inside a try/finally or with statement
                if !self.in_resource_safe_context() {
                    self.resource_issues.push(
                        ResourceManagementIssue::ResourceLeak {
                            resource_type: self.identify_resource_type(func),
                            acquisition_site: self.extract_location(stmt),
                            leak_site: self.extract_location(stmt),
                            cleanup_suggestion: "Use 'with' statement".to_string(),
                        }
                    );
                }
            }
        }
    }
}

fn is_resource_acquisition(&self, func: &Expr) -> bool {
    if let ExprKind::Name { id, .. } = &func.node {
        matches!(id.as_str(), "open" | "connect" | "socket" | "urlopen")
    } else {
        false
    }
}
```

### Testing Pattern Detection

#### Test Without Assertions
```rust
fn check_test_assertions(&mut self, func_def: &Stmt) {
    if let StmtKind::FunctionDef { name, body, .. } = &func_def.node {
        if name.starts_with("test_") || name.starts_with("Test") {
            let has_assertion = body.iter().any(|stmt| {
                self.contains_assertion(stmt)
            });
            
            if !has_assertion {
                self.testing_issues.push(
                    TestingAntiPattern::MissingAssertion {
                        test_name: name.clone(),
                        test_framework: self.detect_test_framework(),
                        location: self.extract_location(func_def),
                    }
                );
            }
        }
    }
}

fn contains_assertion(&self, stmt: &Stmt) -> bool {
    match &stmt.node {
        StmtKind::Assert { .. } => true,
        StmtKind::Expr { value } => {
            if let ExprKind::Call { func, .. } = &value.node {
                self.is_assertion_call(func)
            } else {
                false
            }
        }
        _ => false
    }
}

fn is_assertion_call(&self, func: &Expr) -> bool {
    // Check for unittest assertions, pytest assertions, etc.
    if let ExprKind::Attribute { attr, .. } = &func.node {
        attr.starts_with("assert") || attr == "fail"
    } else {
        false
    }
}
```

### Location Extraction

```rust
impl PythonDetectorVisitor {
    fn extract_location(&self, node: impl HasLocation) -> SourceLocation {
        let location = node.location();
        SourceLocation {
            line: location.row(),
            column: Some(location.column()),
            end_line: node.end_location().map(|l| l.row()),
            end_column: node.end_location().map(|l| l.column()),
            confidence: LocationConfidence::Exact,
        }
    }
}
```

## Integration with Existing Python Analyzer

```rust
// In src/analyzers/python.rs
impl Analyzer for PythonAnalyzer {
    fn analyze(&self, ast: &Ast) -> FileMetrics {
        match ast {
            Ast::Python(python_ast) => {
                let mut metrics = analyze_python_file(python_ast, self.complexity_threshold);
                
                // Add comprehensive detector analysis
                let mut detector = PythonDetectorVisitor::new(
                    &python_ast.source,
                    &python_ast.path
                );
                detector.visit_module(&python_ast.module);
                
                // Convert detected patterns to debt items
                metrics.debt_items.extend(detector.to_debt_items());
                
                metrics
            }
            _ => // ...
        }
    }
}
```

## Python-Specific Patterns to Detect

### Performance Anti-patterns
1. **Global Variable Access in Loops**: Expensive namespace lookups
2. **Repeated Regex Compilation**: `re.search()` without `re.compile()`
3. **List Operations for Set Membership**: `if x in list` vs `if x in set`
4. **Inefficient DataFrame Operations**: Row-wise pandas operations
5. **Missing Generator Usage**: Creating full lists when iteration suffices

### Organization Anti-patterns
1. **Mutable Default Arguments**: `def func(x=[])`
2. **Class Variables vs Instance Variables**: Confusion and bugs
3. **Missing `__slots__`**: For classes with many instances
4. **Circular Imports**: Module dependency cycles
5. **Global State Mutation**: Functions with side effects

### Security Vulnerabilities
1. **Pickle with Untrusted Data**: Arbitrary code execution
2. **Assert for Validation**: Disabled with `-O` flag
3. **Hardcoded Secrets**: API keys, passwords
4. **Path Traversal**: Unsafe path joins
5. **YAML Load**: Using `yaml.load()` instead of `yaml.safe_load()`

### Resource Management Issues
1. **Missing `finally` Blocks**: Resource cleanup not guaranteed
2. **Thread/Process Pool Leaks**: Not shutting down executors
3. **Database Connection Leaks**: Not closing connections
4. **File Descriptor Leaks**: Opening files without closing
5. **Memory Leaks**: Circular references preventing GC

### Testing Anti-patterns
1. **Test Pollution**: Shared mutable state between tests
2. **Missing Fixtures**: Duplicated setup code
3. **Time-Dependent Tests**: Using `time.sleep()` or real time
4. **Missing Mocks**: Testing with real external dependencies
5. **Overly Complex Tests**: Tests harder to understand than code

## Performance Optimizations

1. **AST Caching**: Cache parsed ASTs for unchanged files
2. **Parallel Detection**: Run detectors in parallel per file
3. **Early Termination**: Stop checking once threshold reached
4. **Pattern Compilation**: Pre-compile regex patterns
5. **Incremental Analysis**: Only re-analyze changed functions/classes

## Success Metrics

1. **Detection Rate**: Catch 90% of patterns found by specialized Python tools
2. **False Positive Rate**: < 5% false positives
3. **Performance**: < 50ms per file overhead
4. **Coverage**: Support Python 3.8+ syntax completely