pub mod critical_path;
pub mod dependency;
pub mod git_history;

use anyhow::Result;
use im::HashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    Performance,
    Compliance,
}

/// Aggregates context from multiple providers
pub struct ContextAggregator {
    providers: Vec<Box<dyn ContextProvider>>,
    cache: HashMap<String, ContextMap>,
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
            cache: HashMap::new(),
        }
    }

    pub fn with_provider(mut self, provider: Box<dyn ContextProvider>) -> Self {
        self.providers.push(provider);
        self
    }

    pub fn analyze(&mut self, target: &AnalysisTarget) -> ContextMap {
        let cache_key = format!("{}:{}", target.file_path.display(), target.function_name);

        if let Some(cached) = self.cache.get(&cache_key) {
            return cached.clone();
        }

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

        self.cache.insert(cache_key, context_map.clone());
        context_map
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
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
        let context_contribution = context_map.total_contribution();
        let contextual_risk = (base_risk * (1.0 + context_contribution)).min(10.0);

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
