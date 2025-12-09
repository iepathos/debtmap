pub mod critical_path;
pub mod dependency;
pub mod git_history;

use anyhow::Result;
use dashmap::DashMap;
use im::HashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

/// Trait for context providers that gather additional risk-relevant information
pub trait ContextProvider: Send + Sync {
    /// Name of this context provider
    fn name(&self) -> &str;

    /// Gather context for the given analysis target
    fn gather(&self, target: &AnalysisTarget) -> Result<Context>;

    /// Weight of this provider's contribution to overall risk
    fn weight(&self) -> f64;

    /// Explain the context's contribution to risk
    fn explain(&self, context: &Context) -> String;
}

/// Target for context analysis
#[derive(Debug, Clone)]
pub struct AnalysisTarget {
    pub root_path: PathBuf,
    pub file_path: PathBuf,
    pub function_name: String,
    pub line_range: (usize, usize),
}

/// Context information gathered by a provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    pub provider: String,
    pub weight: f64,
    pub contribution: f64,
    pub details: ContextDetails,
}

/// Detailed context information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContextDetails {
    CriticalPath {
        entry_points: Vec<String>,
        path_weight: f64,
        is_user_facing: bool,
    },
    DependencyChain {
        depth: usize,
        propagated_risk: f64,
        dependents: Vec<String>,
        blast_radius: usize,
    },
    Historical {
        change_frequency: f64,
        bug_density: f64,
        age_days: u32,
        author_count: usize,
    },
    Business {
        priority: Priority,
        impact: Impact,
        annotations: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Impact {
    Revenue,
    UserExperience,
    Security,
    Compliance,
}

/// Thread-safe aggregator for context providers.
///
/// Uses lock-free DashMap for caching to enable safe concurrent access
/// from parallel analysis workers. The aggregator itself is wrapped in
/// Arc for cheap cloning across threads.
///
/// # Thread Safety
///
/// Safe to share across threads via Arc. The internal cache uses DashMap
/// for lock-free concurrent access, avoiding contention in hot paths.
pub struct ContextAggregator {
    providers: Vec<Box<dyn ContextProvider>>,
    cache: Arc<DashMap<String, ContextMap>>,
}

impl Default for ContextAggregator {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextAggregator {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            cache: Arc::new(DashMap::new()),
        }
    }

    pub fn with_provider(mut self, provider: Box<dyn ContextProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    /// Analyze the target and return context information.
    ///
    /// This method uses interior mutability via DashMap for lock-free caching,
    /// so it can be called with &self from multiple threads safely.
    pub fn analyze(&self, target: &AnalysisTarget) -> ContextMap {
        let cache_key = format!("{}:{}", target.file_path.display(), target.function_name);

        // Check cache (lock-free read)
        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

        // Gather context from providers
        let mut context_map = ContextMap::new();
        for provider in &self.providers {
            match provider.gather(target) {
                Ok(context) => {
                    context_map.add(provider.name().to_string(), context);
                }
                Err(e) => {
                    log::debug!("Context provider {} failed: {}", provider.name(), e);
                }
            }
        }

        // Insert into cache (lock-free write)
        self.cache.insert(cache_key, context_map.clone());
        context_map
    }

    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}

impl Clone for ContextAggregator {
    fn clone(&self) -> Self {
        Self {
            providers: Vec::new(),          // Don't clone providers (they're heavy)
            cache: Arc::clone(&self.cache), // Share cache via Arc
        }
    }
}

/// Map of contexts from various providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMap {
    contexts: HashMap<String, Context>,
}

impl Default for ContextMap {
    fn default() -> Self {
        Self::new()
    }
}

impl ContextMap {
    pub fn new() -> Self {
        Self {
            contexts: HashMap::new(),
        }
    }

    pub fn add(&mut self, provider: String, context: Context) {
        self.contexts.insert(provider, context);
    }

    pub fn get(&self, provider: &str) -> Option<&Context> {
        self.contexts.get(provider)
    }

    pub fn total_contribution(&self) -> f64 {
        self.contexts
            .values()
            .map(|c| c.contribution * c.weight)
            .sum()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Context)> {
        self.contexts.iter()
    }
}

/// Enhanced risk information with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextualRisk {
    pub base_risk: f64,
    pub contextual_risk: f64,
    pub contexts: Vec<Context>,
    pub explanation: String,
}

impl ContextualRisk {
    pub fn new(base_risk: f64, context_map: &ContextMap) -> Self {
        let raw_contribution = context_map.total_contribution();

        // Cap contribution at 2.0 to prevent excessive score amplification
        // Without cap: contribution=15+ → 16x multiplier → inflated scores
        // With cap: contribution capped at 2.0 → max 3x multiplier → reasonable prioritization
        // This ensures high-churn files get elevated priority without absurd inflation
        let context_contribution = raw_contribution.min(2.0);

        let contextual_risk = base_risk * (1.0 + context_contribution);

        let contexts: Vec<Context> = context_map
            .iter()
            .map(|(_, context)| context.clone())
            .collect();

        let explanation = Self::generate_explanation(base_risk, &contexts);

        Self {
            base_risk,
            contextual_risk,
            contexts,
            explanation,
        }
    }

    fn generate_explanation(base_risk: f64, contexts: &[Context]) -> String {
        let mut parts = vec![format!("Base risk: {:.1}", base_risk)];

        for context in contexts {
            if context.contribution > 0.1 {
                parts.push(format!(
                    "{}: +{:.1}",
                    context.provider,
                    context.contribution * context.weight
                ));
            }
        }

        parts.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_context_aggregator_concurrent_access() {
        let aggregator = Arc::new(ContextAggregator::new());
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let agg = Arc::clone(&aggregator);
                thread::spawn(move || {
                    let target = AnalysisTarget {
                        root_path: PathBuf::from("/test"),
                        file_path: PathBuf::from(format!("/test/file{}.rs", i)),
                        function_name: format!("test_fn_{}", i),
                        line_range: (1, 10),
                    };
                    agg.analyze(&target)
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }
        // No panics = success
    }

    /// Stress test that simulates analyzing a large codebase like debtmap itself.
    /// This should reproduce stack overflow if recursive DFS is used.
    #[test]
    fn test_large_call_graph_no_stack_overflow() {
        use crate::priority::call_graph::CallGraph;
        use crate::risk::context::critical_path::{CriticalPathAnalyzer, CriticalPathProvider};

        // Build a call graph with 4000 functions (similar to debtmap)
        let mut call_graph = CallGraph::new();
        let num_functions = 4000;

        // Create a deep call chain: main -> f1 -> f2 -> ... -> f3999
        for i in 0..num_functions - 1 {
            let caller = format!("func_{}", i);
            let callee = format!("func_{}", i + 1);
            call_graph.add_edge_by_name(caller, callee, PathBuf::from("src/lib.rs"));
        }

        // Add main as entry point
        let mut analyzer = CriticalPathAnalyzer::new();
        analyzer.call_graph = call_graph;
        analyzer.entry_points.push_back(super::critical_path::EntryPoint {
            function_name: "func_0".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: super::critical_path::EntryType::Main,
            is_user_facing: true,
        });

        let provider = CriticalPathProvider::new(analyzer);

        // This should NOT stack overflow - if it does, we found the bug
        let target = AnalysisTarget {
            root_path: PathBuf::from("/project"),
            function_name: "func_2000".to_string(), // Middle of the chain
            file_path: PathBuf::from("src/lib.rs"),
            line_range: (1, 10),
        };

        let result = provider.gather(&target);
        assert!(result.is_ok(), "gather should succeed without stack overflow");
    }

    /// Test the full context aggregator with all providers on a large graph
    #[test]
    fn test_context_aggregator_large_codebase() {
        use crate::risk::context::critical_path::{CriticalPathAnalyzer, CriticalPathProvider};
        use crate::risk::context::dependency::{DependencyGraph, DependencyRiskProvider};

        // Create critical path provider with large graph
        let mut call_graph = crate::priority::call_graph::CallGraph::new();
        for i in 0..1000 {
            let caller = format!("func_{}", i);
            let callee = format!("func_{}", i + 1);
            call_graph.add_edge_by_name(caller, callee, PathBuf::from("src/lib.rs"));
        }

        let mut cp_analyzer = CriticalPathAnalyzer::new();
        cp_analyzer.call_graph = call_graph;
        cp_analyzer.entry_points.push_back(super::critical_path::EntryPoint {
            function_name: "func_0".to_string(),
            file_path: PathBuf::from("src/main.rs"),
            entry_type: super::critical_path::EntryType::Main,
            is_user_facing: true,
        });

        // Create dependency provider
        let dep_graph = DependencyGraph::new();

        // Build aggregator with both providers
        let aggregator = ContextAggregator::new()
            .with_provider(Box::new(CriticalPathProvider::new(cp_analyzer)))
            .with_provider(Box::new(DependencyRiskProvider::new(dep_graph)));

        // Analyze 100 different functions - should NOT overflow
        for i in 0..100 {
            let target = AnalysisTarget {
                root_path: PathBuf::from("/project"),
                function_name: format!("func_{}", i * 10),
                file_path: PathBuf::from("src/lib.rs"),
                line_range: (1, 10),
            };

            let context_map = aggregator.analyze(&target);
            // Just verify we get a result without crashing
            let _ = context_map.total_contribution();
        }
    }
}
