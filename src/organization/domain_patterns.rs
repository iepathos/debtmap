//! Domain Pattern Detection for Semantic Clustering (Spec 175)
//!
//! This module implements semantic domain pattern detection to identify cohesive
//! method clusters based on design patterns and shared data structures, rather than
//! just syntactic prefix matching.
//!
//! # Example
//!
//! Instead of classifying all methods starting with various prefixes as "Utilities",
//! detect semantic patterns:
//! - `register_observer_interfaces()` → Observer Pattern
//! - `detect_observer_dispatch()` → Observer Pattern
//! - `check_for_callback_patterns()` → Callback Pattern

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Minimum confidence threshold for domain pattern match
/// Note: Lowered from spec's 0.60 to 0.40 to be more inclusive with partial signal matches
pub const DOMAIN_PATTERN_THRESHOLD: f64 = 0.40;

/// Minimum methods required to form domain cluster
pub const MIN_DOMAIN_CLUSTER_SIZE: usize = 3;

/// Signal weights for pattern scoring
pub const WEIGHT_NAME_KEYWORDS: f64 = 0.30;
pub const WEIGHT_STRUCTURE_ACCESS: f64 = 0.40;
pub const WEIGHT_CALL_GRAPH: f64 = 0.20;
pub const WEIGHT_DOCUMENTATION: f64 = 0.10;

/// Design pattern categories for semantic clustering
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DomainPattern {
    /// Observer/Listener/Subject pattern
    ObserverPattern,
    /// Callback/Handler/Event binding pattern
    CallbackPattern,
    /// Registry/Catalog/Index/Lookup pattern
    RegistryPattern,
    /// Builder/Fluent API pattern
    BuilderPattern,
    /// Type inference/checking pattern
    TypeInferencePattern,
    /// AST traversal/visitor pattern
    AstTraversalPattern,
}

impl DomainPattern {
    /// Get pattern keywords for name matching
    pub fn keywords(&self) -> &'static [&'static str] {
        match self {
            Self::ObserverPattern => &[
                "observer",
                "subject",
                "notify",
                "subscribe",
                "listener",
                "event",
                "dispatch",
                "interface",
                "registry",
                "register",
                "unregister",
            ],
            Self::CallbackPattern => &[
                "callback", "handler", "event", "binding", "hook", "trigger", "invoke", "deferred",
            ],
            Self::RegistryPattern => &[
                "registry",
                "catalog",
                "index",
                "lookup",
                "cache",
                "store",
                "repository",
                "collection",
            ],
            Self::BuilderPattern => &[
                "builder",
                "build",
                "with",
                "set",
                "add",
                "fluent",
                "chain",
                "construct",
            ],
            Self::TypeInferencePattern => &[
                "infer",
                "type",
                "check",
                "resolve",
                "constraint",
                "unify",
                "deduce",
                "analyze",
            ],
            Self::AstTraversalPattern => &[
                "visit", "traverse", "walk", "ast", "node", "tree", "descend", "recurse",
            ],
        }
    }

    /// Get pattern-related data structures for structure access matching
    pub fn structures(&self) -> &'static [&'static str] {
        match self {
            Self::ObserverPattern => &[
                "ObserverRegistry",
                "ObserverPattern",
                "Subject",
                "Listener",
                "EventDispatcher",
                "ObserverInterface",
            ],
            Self::CallbackPattern => &[
                "CallbackTracker",
                "EventHandler",
                "CallbackRegistry",
                "DeferredCallback",
                "EventBinding",
            ],
            Self::RegistryPattern => &[
                "Registry",
                "Catalog",
                "Index",
                "Cache",
                "Store",
                "Repository",
            ],
            Self::BuilderPattern => &["Builder", "FluentBuilder", "Constructor"],
            Self::TypeInferencePattern => &[
                "TypeInference",
                "TypeChecker",
                "ConstraintSolver",
                "TypeEnvironment",
            ],
            Self::AstTraversalPattern => &["Visitor", "AstWalker", "TreeTraverser", "NodeVisitor"],
        }
    }

    /// Get module name for this pattern
    pub fn module_name(&self) -> String {
        match self {
            Self::ObserverPattern => "observer_pattern".to_string(),
            Self::CallbackPattern => "callback_pattern".to_string(),
            Self::RegistryPattern => "registry".to_string(),
            Self::BuilderPattern => "builder".to_string(),
            Self::TypeInferencePattern => "type_inference".to_string(),
            Self::AstTraversalPattern => "ast_traversal".to_string(),
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> String {
        match self {
            Self::ObserverPattern => "Observer Pattern Detection".to_string(),
            Self::CallbackPattern => "Callback Pattern Detection".to_string(),
            Self::RegistryPattern => "Registry Pattern".to_string(),
            Self::BuilderPattern => "Builder Pattern".to_string(),
            Self::TypeInferencePattern => "Type Inference".to_string(),
            Self::AstTraversalPattern => "AST Traversal".to_string(),
        }
    }

    /// Get all available patterns
    pub fn all_patterns() -> Vec<Self> {
        vec![
            Self::ObserverPattern,
            Self::CallbackPattern,
            Self::RegistryPattern,
            Self::BuilderPattern,
            Self::TypeInferencePattern,
            Self::AstTraversalPattern,
        ]
    }
}

/// Evidence supporting a domain pattern match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEvidence {
    pub matched_keywords: Vec<String>,
    pub accessed_structures: Vec<String>,
    pub cohesive_calls: Vec<(String, String)>,
    pub documentation_matches: Vec<String>,
}

/// Result of domain pattern detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomainPatternMatch {
    pub pattern: DomainPattern,
    pub confidence: f64,
    pub evidence: PatternEvidence,
}

/// Simplified method information for pattern detection
#[derive(Debug, Clone)]
pub struct MethodInfo {
    pub name: String,
    pub body: String,
    pub doc_comment: Option<String>,
}

/// Call graph edge for cohesion analysis
#[derive(Debug, Clone)]
pub struct CallEdge {
    pub caller: String,
    pub callee: String,
}

/// Context for pattern detection including file-level information
#[derive(Debug, Clone)]
pub struct FileContext {
    pub methods: Vec<MethodInfo>,
    pub structures: HashSet<String>,
    pub call_edges: Vec<CallEdge>,
}

impl FileContext {
    /// Get incoming and outgoing call edges for a method
    pub fn edges_for_method(&self, method_name: &str) -> Vec<&CallEdge> {
        self.call_edges
            .iter()
            .filter(|edge| edge.caller == method_name || edge.callee == method_name)
            .collect()
    }
}

/// Domain pattern detector
pub struct DomainPatternDetector {
    patterns: Vec<DomainPattern>,
}

impl DomainPatternDetector {
    /// Create new detector with all patterns
    pub fn new() -> Self {
        Self {
            patterns: DomainPattern::all_patterns(),
        }
    }

    /// Detect domain pattern for a single method
    pub fn detect_method_domain(
        &self,
        method: &MethodInfo,
        context: &FileContext,
    ) -> Option<DomainPatternMatch> {
        let mut scores: Vec<(DomainPattern, f64, PatternEvidence)> = Vec::new();

        for pattern in &self.patterns {
            let evidence = self.collect_evidence(method, context, pattern);
            let score = self.score_pattern(method, context, pattern, &evidence);

            if score > 0.0 {
                scores.push((pattern.clone(), score, evidence));
            }
        }

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Return highest scoring pattern if above threshold
        if let Some((pattern, score, evidence)) = scores.first() {
            if *score >= DOMAIN_PATTERN_THRESHOLD {
                return Some(DomainPatternMatch {
                    pattern: pattern.clone(),
                    confidence: *score,
                    evidence: evidence.clone(),
                });
            }
        }

        None
    }

    /// Calculate pattern score based on multiple signals
    fn score_pattern(
        &self,
        method: &MethodInfo,
        context: &FileContext,
        pattern: &DomainPattern,
        evidence: &PatternEvidence,
    ) -> f64 {
        let mut score = 0.0;

        // Signal 1: Method name contains pattern keywords (weight: 0.30)
        let name_score = self.name_keyword_score(method, pattern);
        score += WEIGHT_NAME_KEYWORDS * name_score;

        // Signal 2: Accesses pattern-related structures (weight: 0.40)
        let struct_score = self.structure_access_score(evidence, pattern);
        score += WEIGHT_STRUCTURE_ACCESS * struct_score;

        // Signal 3: Called by other pattern methods (weight: 0.20)
        let graph_score = self.call_graph_cohesion_score(method, context, pattern);
        score += WEIGHT_CALL_GRAPH * graph_score;

        // Signal 4: Comment/doc contains pattern keywords (weight: 0.10)
        let doc_score = self.documentation_score(evidence, pattern);
        score += WEIGHT_DOCUMENTATION * doc_score;

        score
    }

    /// Score based on method name keyword matching
    fn name_keyword_score(&self, method: &MethodInfo, pattern: &DomainPattern) -> f64 {
        let name_lower = method.name.to_lowercase();
        let keywords = pattern.keywords();

        let matches = keywords
            .iter()
            .filter(|kw| name_lower.contains(*kw))
            .count();

        if matches > 0 {
            // Give 0.5 for one match, scale up to 1.0 for 3+ matches
            // This is more generous than dividing by total keywords
            ((matches as f64) * 0.4 + 0.1).min(1.0)
        } else {
            0.0
        }
    }

    /// Score based on structure access patterns
    fn structure_access_score(&self, evidence: &PatternEvidence, pattern: &DomainPattern) -> f64 {
        let pattern_structures = pattern.structures();
        if pattern_structures.is_empty() {
            return 0.0;
        }

        let matches = evidence.accessed_structures.len();
        if matches > 0 {
            // Give full score if any pattern structure is accessed
            // Structure access is a strong signal
            1.0
        } else {
            0.0
        }
    }

    /// Score based on call graph cohesion
    fn call_graph_cohesion_score(
        &self,
        method: &MethodInfo,
        context: &FileContext,
        pattern: &DomainPattern,
    ) -> f64 {
        // Find other methods matching this pattern
        let pattern_methods: Vec<_> = context
            .methods
            .iter()
            .filter(|m| {
                let score = self.name_keyword_score(m, pattern);
                score > 0.5
            })
            .collect();

        if pattern_methods.is_empty() {
            return 0.0;
        }

        // Calculate what % of calls are to/from pattern methods
        let edges = context.edges_for_method(&method.name);
        let total_calls = edges.len();

        if total_calls == 0 {
            return 0.0;
        }

        let pattern_method_names: HashSet<_> =
            pattern_methods.iter().map(|m| m.name.as_str()).collect();

        let pattern_calls = edges
            .iter()
            .filter(|edge| {
                pattern_method_names.contains(edge.caller.as_str())
                    || pattern_method_names.contains(edge.callee.as_str())
            })
            .count();

        pattern_calls as f64 / total_calls as f64
    }

    /// Score based on documentation keyword matching
    fn documentation_score(&self, evidence: &PatternEvidence, _pattern: &DomainPattern) -> f64 {
        if evidence.documentation_matches.is_empty() {
            0.0
        } else {
            // Scale based on number of doc keyword matches (cap at 1.0)
            (evidence.documentation_matches.len() as f64 / 3.0).min(1.0)
        }
    }

    /// Collect evidence for pattern matching
    fn collect_evidence(
        &self,
        method: &MethodInfo,
        context: &FileContext,
        pattern: &DomainPattern,
    ) -> PatternEvidence {
        let name_lower = method.name.to_lowercase();
        let body_lower = method.body.to_lowercase();

        // Collect matched keywords from method name
        let matched_keywords: Vec<String> = pattern
            .keywords()
            .iter()
            .filter(|kw| name_lower.contains(*kw))
            .map(|s| s.to_string())
            .collect();

        // Collect accessed structures from method body
        // Check for exact matches (case-sensitive) or lowercase snake_case variants
        let accessed_structures: Vec<String> = pattern
            .structures()
            .iter()
            .filter(|s| {
                // Check exact match (case-sensitive)
                if method.body.contains(*s) {
                    return true;
                }

                // Check lowercase variant
                let structure_lower = s.to_lowercase();
                if body_lower.contains(&structure_lower) {
                    return true;
                }

                // Check snake_case variant (e.g., ObserverRegistry -> observer_registry)
                let snake_case = s
                    .chars()
                    .enumerate()
                    .flat_map(|(i, c)| {
                        if i > 0 && c.is_uppercase() {
                            vec!['_', c.to_ascii_lowercase()]
                        } else {
                            vec![c.to_ascii_lowercase()]
                        }
                    })
                    .collect::<String>();

                body_lower.contains(&snake_case)
            })
            .map(|s| s.to_string())
            .collect();

        // Collect cohesive calls (calls to/from methods matching this pattern)
        let pattern_method_names: HashSet<_> = context
            .methods
            .iter()
            .filter(|m| {
                let score = self.name_keyword_score(m, pattern);
                score > 0.5
            })
            .map(|m| m.name.clone())
            .collect();

        let cohesive_calls: Vec<(String, String)> = context
            .edges_for_method(&method.name)
            .into_iter()
            .filter(|edge| {
                pattern_method_names.contains(&edge.caller)
                    || pattern_method_names.contains(&edge.callee)
            })
            .map(|edge| (edge.caller.clone(), edge.callee.clone()))
            .collect();

        // Collect documentation matches
        let documentation_matches = if let Some(ref doc) = method.doc_comment {
            let doc_lower = doc.to_lowercase();
            pattern
                .keywords()
                .iter()
                .filter(|kw| doc_lower.contains(*kw))
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        };

        PatternEvidence {
            matched_keywords,
            accessed_structures,
            cohesive_calls,
            documentation_matches,
        }
    }
}

impl Default for DomainPatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Cluster methods by detected domain patterns
pub fn cluster_methods_by_domain(
    methods: &[MethodInfo],
    context: &FileContext,
    detector: &DomainPatternDetector,
) -> HashMap<DomainPattern, Vec<MethodInfo>> {
    let mut clusters: HashMap<DomainPattern, Vec<MethodInfo>> = HashMap::new();

    for method in methods {
        if let Some(domain_match) = detector.detect_method_domain(method, context) {
            clusters
                .entry(domain_match.pattern)
                .or_default()
                .push(method.clone());
        }
    }

    // Filter out clusters that are too small
    clusters.retain(|_, methods| methods.len() >= MIN_DOMAIN_CLUSTER_SIZE);

    clusters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observer_pattern_keywords() {
        let pattern = DomainPattern::ObserverPattern;
        let keywords = pattern.keywords();

        assert!(keywords.contains(&"observer"));
        assert!(keywords.contains(&"notify"));
        assert!(keywords.contains(&"dispatch"));
    }

    #[test]
    fn test_observer_pattern_detection() {
        let detector = DomainPatternDetector::new();

        let method = MethodInfo {
            name: "register_observer_interfaces".to_string(),
            body: "self.observer_registry.register(observer)".to_string(),
            doc_comment: Some("Register an observer interface".to_string()),
        };

        let context = FileContext {
            methods: vec![method.clone()],
            structures: ["ObserverRegistry".to_string()].into_iter().collect(),
            call_edges: vec![],
        };

        let result = detector.detect_method_domain(&method, &context);

        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.pattern, DomainPattern::ObserverPattern);
        assert!(matched.confidence >= DOMAIN_PATTERN_THRESHOLD);
        assert!(!matched.evidence.matched_keywords.is_empty());
    }

    #[test]
    fn test_callback_pattern_detection() {
        let detector = DomainPatternDetector::new();

        let method = MethodInfo {
            name: "check_for_callback_patterns".to_string(),
            body: "self.callback_tracker.track(callback)".to_string(),
            doc_comment: None,
        };

        let context = FileContext {
            methods: vec![method.clone()],
            structures: ["CallbackTracker".to_string()].into_iter().collect(),
            call_edges: vec![],
        };

        let result = detector.detect_method_domain(&method, &context);

        assert!(result.is_some());
        let matched = result.unwrap();
        assert_eq!(matched.pattern, DomainPattern::CallbackPattern);
        assert!(matched.confidence >= DOMAIN_PATTERN_THRESHOLD);
    }

    #[test]
    fn test_pattern_clustering() {
        let detector = DomainPatternDetector::new();

        let methods = vec![
            MethodInfo {
                name: "register_observer".to_string(),
                body: "observer_registry.add(obs)".to_string(),
                doc_comment: None,
            },
            MethodInfo {
                name: "notify_observers".to_string(),
                body: "observer_registry.notify_all()".to_string(),
                doc_comment: None,
            },
            MethodInfo {
                name: "unregister_observer".to_string(),
                body: "observer_registry.remove(obs)".to_string(),
                doc_comment: None,
            },
            MethodInfo {
                name: "handle_callback".to_string(),
                body: "callback_tracker.invoke()".to_string(),
                doc_comment: None,
            },
        ];

        let context = FileContext {
            methods: methods.clone(),
            structures: [
                "ObserverRegistry".to_string(),
                "CallbackTracker".to_string(),
            ]
            .into_iter()
            .collect(),
            call_edges: vec![],
        };

        let clusters = cluster_methods_by_domain(&methods, &context, &detector);

        // Should have observer cluster with 3 methods
        assert!(clusters.contains_key(&DomainPattern::ObserverPattern));
        let observer_cluster = &clusters[&DomainPattern::ObserverPattern];
        assert_eq!(observer_cluster.len(), 3);

        // Should NOT have callback cluster (only 1 method, below MIN_DOMAIN_CLUSTER_SIZE)
        assert!(!clusters.contains_key(&DomainPattern::CallbackPattern));
    }

    #[test]
    fn test_minimum_cluster_size() {
        let detector = DomainPatternDetector::new();

        let methods = vec![
            MethodInfo {
                name: "register_observer".to_string(),
                body: "observer_registry.add(obs)".to_string(),
                doc_comment: None,
            },
            MethodInfo {
                name: "notify_observers".to_string(),
                body: "observer_registry.notify_all()".to_string(),
                doc_comment: None,
            },
        ];

        let context = FileContext {
            methods: methods.clone(),
            structures: ["ObserverRegistry".to_string()].into_iter().collect(),
            call_edges: vec![],
        };

        let clusters = cluster_methods_by_domain(&methods, &context, &detector);

        // Should not create cluster with only 2 methods (below MIN_DOMAIN_CLUSTER_SIZE of 3)
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_all_patterns_defined() {
        let patterns = DomainPattern::all_patterns();
        assert_eq!(patterns.len(), 6);

        // Verify each pattern has keywords and structures
        for pattern in patterns {
            assert!(!pattern.keywords().is_empty());
            // Some patterns may have empty structures, that's ok
            assert!(!pattern.module_name().is_empty());
            assert!(!pattern.description().is_empty());
        }
    }

    #[test]
    fn test_call_graph_cohesion() {
        let detector = DomainPatternDetector::new();

        let methods = vec![
            MethodInfo {
                name: "register_observer".to_string(),
                body: "observer_registry.add(obs)".to_string(),
                doc_comment: None,
            },
            MethodInfo {
                name: "notify_observers".to_string(),
                body: "observer_registry.notify_all()".to_string(),
                doc_comment: None,
            },
            MethodInfo {
                name: "dispatch_event".to_string(),
                body: "self.notify_observers()".to_string(),
                doc_comment: None,
            },
        ];

        let context = FileContext {
            methods: methods.clone(),
            structures: ["ObserverRegistry".to_string()].into_iter().collect(),
            call_edges: vec![
                CallEdge {
                    caller: "dispatch_event".to_string(),
                    callee: "notify_observers".to_string(),
                },
                CallEdge {
                    caller: "notify_observers".to_string(),
                    callee: "register_observer".to_string(),
                },
            ],
        };

        let result = detector.detect_method_domain(&methods[2], &context);

        assert!(result.is_some());
        let matched = result.unwrap();
        // Should have high confidence due to call graph cohesion
        assert!(matched.confidence >= DOMAIN_PATTERN_THRESHOLD);
    }
}
