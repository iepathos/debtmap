/// Unbounded collection growth detection for Python
use super::{
    AffectedScope, GrowthPattern, ImpactLevel, PythonResourceDetector, PythonResourceIssueType,
    ResourceImpact, ResourceIssue, ResourceLocation, ResourceSeverity,
};
use rustpython_parser::ast::{self, Expr, Stmt};
use std::collections::HashMap;
use std::path::Path;

pub struct PythonUnboundedCollectionDetector {
    growth_patterns: Vec<GrowthPatternMatcher>,
    size_thresholds: HashMap<String, usize>,
}

struct GrowthPatternMatcher {
    pattern: GrowthPattern,
    matcher: Box<dyn Fn(&str, &CollectionInfo) -> bool>,
}

#[derive(Debug)]
struct CollectionInfo {
    name: String,
    collection_type: String,
    has_append: bool,
    has_extend: bool,
    has_add: bool,
    has_removal: bool,
    has_clear: bool,
    has_size_check: bool,
    is_global: bool,
    is_class_attribute: bool,
    line: usize,
}

impl PythonUnboundedCollectionDetector {
    pub fn new() -> Self {
        let growth_patterns = vec![
            GrowthPatternMatcher {
                pattern: GrowthPattern::UnboundedAppend,
                matcher: Box::new(|_name, info| {
                    (info.has_append || info.has_extend || info.has_add)
                        && !info.has_removal
                        && !info.has_size_check
                }),
            },
            GrowthPatternMatcher {
                pattern: GrowthPattern::NoEviction,
                matcher: Box::new(|_name, info| {
                    info.collection_type == "dict" && !info.has_removal && !info.has_clear
                }),
            },
            GrowthPatternMatcher {
                pattern: GrowthPattern::MemoryCache,
                matcher: Box::new(|name, info| {
                    (name.contains("cache") || name.contains("memo"))
                        && !info.has_size_check
                        && !info.has_removal
                }),
            },
        ];

        let mut size_thresholds = HashMap::new();
        size_thresholds.insert("list".to_string(), 10000);
        size_thresholds.insert("dict".to_string(), 5000);
        size_thresholds.insert("set".to_string(), 10000);

        Self {
            growth_patterns,
            size_thresholds,
        }
    }

    fn analyze_module(&self, module: &ast::Mod) -> Vec<CollectionInfo> {
        let mut collections = Vec::new();

        if let ast::Mod::Module(module) = module {
            for stmt in &module.body {
                self.analyze_statement(stmt, &mut collections, false, false);
            }
        }

        collections
    }

    fn analyze_statement(
        &self,
        stmt: &Stmt,
        collections: &mut Vec<CollectionInfo>,
        in_class: bool,
        in_function: bool,
    ) {
        match stmt {
            Stmt::Assign(assign) => {
                // Check for collection initialization
                if let Some(collection_type) = self.get_collection_type(&assign.value) {
                    for target in &assign.targets {
                        if let Some(name) = self.extract_name(target) {
                            let mut info = CollectionInfo {
                                name: name.clone(),
                                collection_type: collection_type.clone(),
                                has_append: false,
                                has_extend: false,
                                has_add: false,
                                has_removal: false,
                                has_clear: false,
                                has_size_check: false,
                                is_global: !in_function && !in_class,
                                is_class_attribute: in_class && !in_function,
                                line: 1, // TODO: Track actual line numbers
                            };

                            // Scan for operations on this collection
                            self.scan_for_operations(&name, &mut info);
                            collections.push(info);
                        }
                    }
                }
            }
            Stmt::ClassDef(class_def) => {
                for class_stmt in &class_def.body {
                    self.analyze_statement(class_stmt, collections, true, in_function);
                }
            }
            Stmt::FunctionDef(func_def) => {
                for func_stmt in &func_def.body {
                    self.analyze_statement(func_stmt, collections, in_class, true);
                }
            }
            Stmt::For(for_stmt) => {
                for body_stmt in &for_stmt.body {
                    self.analyze_statement(body_stmt, collections, in_class, in_function);
                }
            }
            Stmt::While(while_stmt) => {
                for body_stmt in &while_stmt.body {
                    self.analyze_statement(body_stmt, collections, in_class, in_function);
                }
            }
            Stmt::If(if_stmt) => {
                for body_stmt in &if_stmt.body {
                    self.analyze_statement(body_stmt, collections, in_class, in_function);
                }
                for else_stmt in &if_stmt.orelse {
                    self.analyze_statement(else_stmt, collections, in_class, in_function);
                }
            }
            Stmt::Expr(expr_stmt) => {
                // Check for method calls on collections
                if let Expr::Call(call) = expr_stmt.value.as_ref() {
                    if let Expr::Attribute(attr) = call.func.as_ref() {
                        let method_name = attr.attr.to_string();
                        if let Some(collection_name) = self.extract_name(attr.value.as_ref()) {
                            // Update collection info if we find it
                            for info in collections.iter_mut() {
                                if info.name == collection_name {
                                    self.update_collection_info(info, &method_name);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn get_collection_type(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::List(_) => Some("list".to_string()),
            Expr::Dict(_) => Some("dict".to_string()),
            Expr::Set(_) => Some("set".to_string()),
            Expr::Call(call) => {
                if let Expr::Name(name) = call.func.as_ref() {
                    match name.id.as_str() {
                        "list" => Some("list".to_string()),
                        "dict" => Some("dict".to_string()),
                        "set" => Some("set".to_string()),
                        "defaultdict" => Some("dict".to_string()),
                        "OrderedDict" => Some("dict".to_string()),
                        "Counter" => Some("dict".to_string()),
                        _ => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn extract_name(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => Some(name.id.to_string()),
            Expr::Attribute(attr) => {
                if let Expr::Name(name) = attr.value.as_ref() {
                    if &name.id == "self" {
                        Some(format!("self.{}", attr.attr))
                    } else {
                        Some(format!("{}.{}", name.id, attr.attr))
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn scan_for_operations(&self, _collection_name: &str, _info: &mut CollectionInfo) {
        // In a real implementation, we would scan the entire module for operations
        // For now, we'll mark some as having operations based on patterns
    }

    fn update_collection_info(&self, info: &mut CollectionInfo, method_name: &str) {
        match method_name {
            "append" | "insert" => info.has_append = true,
            "extend" | "update" => info.has_extend = true,
            "add" => info.has_add = true,
            "remove" | "pop" | "discard" | "popitem" => info.has_removal = true,
            "clear" => info.has_clear = true,
            _ => {}
        }
    }

    fn detect_unbounded_collections(&self, collections: &[CollectionInfo]) -> Vec<ResourceIssue> {
        let mut issues = Vec::new();

        for collection in collections {
            // Check if collection is likely unbounded
            for pattern_matcher in &self.growth_patterns {
                if (pattern_matcher.matcher)(&collection.name, collection) {
                    let severity = if collection.is_global {
                        ResourceSeverity::High
                    } else if collection.is_class_attribute {
                        ResourceSeverity::Medium
                    } else {
                        ResourceSeverity::Low
                    };

                    issues.push(ResourceIssue {
                        issue_type: PythonResourceIssueType::UnboundedCollection {
                            collection_name: collection.name.clone(),
                            growth_pattern: pattern_matcher.pattern.clone(),
                        },
                        severity,
                        location: ResourceLocation {
                            line: collection.line,
                            column: 0,
                            end_line: None,
                            end_column: None,
                        },
                        suggestion: self.get_bounding_suggestion(
                            &collection.collection_type,
                            &pattern_matcher.pattern,
                        ),
                    });
                }
            }
        }

        issues
    }

    fn get_bounding_suggestion(&self, collection_type: &str, pattern: &GrowthPattern) -> String {
        match pattern {
            GrowthPattern::UnboundedAppend => {
                format!(
                    "Consider implementing a size limit for the {}. Use collections.deque with maxlen or implement custom eviction.",
                    collection_type
                )
            }
            GrowthPattern::NoEviction => {
                "Implement cache eviction policy (LRU, TTL, or size-based). Consider using functools.lru_cache or cachetools.".to_string()
            }
            GrowthPattern::MemoryCache => {
                "Use bounded caching with functools.lru_cache(maxsize=N) or implement size/time-based eviction.".to_string()
            }
            GrowthPattern::RecursiveAccumulation => {
                "Avoid recursive data accumulation. Consider iterative approaches or clear data between iterations.".to_string()
            }
        }
    }
}

impl PythonResourceDetector for PythonUnboundedCollectionDetector {
    fn detect_issues(&self, module: &ast::Mod, _path: &Path) -> Vec<ResourceIssue> {
        let collections = self.analyze_module(module);
        self.detect_unbounded_collections(&collections)
    }

    fn assess_resource_impact(&self, issue: &ResourceIssue) -> ResourceImpact {
        let impact_level = match &issue.severity {
            ResourceSeverity::Critical => ImpactLevel::Critical,
            ResourceSeverity::High => ImpactLevel::High,
            ResourceSeverity::Medium => ImpactLevel::Medium,
            ResourceSeverity::Low => ImpactLevel::Low,
        };

        ResourceImpact {
            impact_level,
            affected_scope: match &issue.issue_type {
                PythonResourceIssueType::UnboundedCollection { .. } => AffectedScope::Module,
                _ => AffectedScope::Function,
            },
            estimated_severity: match impact_level {
                ImpactLevel::Critical => 1.0,
                ImpactLevel::High => 0.7,
                ImpactLevel::Medium => 0.5,
                ImpactLevel::Low => 0.3,
            },
        }
    }
}
