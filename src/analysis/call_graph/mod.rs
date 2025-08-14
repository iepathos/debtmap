//! Enhanced Call Graph Analysis Module
//!
//! This module provides advanced call graph analysis that addresses the false positives
//! in dead code detection by tracking trait implementations, function pointers, closures,
//! framework patterns, and cross-module dependencies.
//!
//! The analysis is performed in multiple phases:
//! 1. Basic call graph construction (existing functionality)
//! 2. Trait method resolution
//! 3. Function pointer and closure tracking
//! 4. Framework pattern recognition
//! 5. Cross-module dependency analysis

use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use anyhow::Result;
use im::HashSet;
use std::path::{Path, PathBuf};
use syn::File;

mod cross_module;
mod framework_patterns;
mod function_pointer;
mod trait_registry;

pub use cross_module::{CrossModuleTracker, ModuleBoundary, PublicApiInfo};
pub use framework_patterns::{FrameworkPattern, FrameworkPatternDetector, PatternType};
pub use function_pointer::{ClosureInfo, FunctionPointerInfo, FunctionPointerTracker};
pub use trait_registry::{TraitImplementation, TraitMethod, TraitRegistry};

/// Enhanced call graph that includes trait dispatch, function pointers, and framework patterns
#[derive(Debug, Clone)]
pub struct EnhancedCallGraph {
    /// Base call graph with direct function calls
    pub base_graph: CallGraph,
    /// Trait registry for tracking trait implementations and dispatch
    pub trait_registry: TraitRegistry,
    /// Function pointer tracker for closures and higher-order functions
    pub function_pointer_tracker: FunctionPointerTracker,
    /// Framework pattern detector for test functions, handlers, etc.
    pub framework_patterns: FrameworkPatternDetector,
    /// Cross-module dependency tracker
    pub cross_module_tracker: CrossModuleTracker,
}

/// Configuration for enhanced call graph analysis
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// Enable trait method resolution
    pub enable_trait_analysis: bool,
    /// Enable function pointer tracking
    pub enable_function_pointer_tracking: bool,
    /// Enable framework pattern detection
    pub enable_framework_patterns: bool,
    /// Enable cross-module analysis
    pub enable_cross_module_analysis: bool,
    /// Maximum depth for transitive analysis
    pub max_analysis_depth: usize,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            enable_trait_analysis: true,
            enable_function_pointer_tracking: true,
            enable_framework_patterns: true,
            enable_cross_module_analysis: true,
            max_analysis_depth: 10,
        }
    }
}

/// Builder for enhanced call graph analysis
pub struct EnhancedCallGraphBuilder {
    config: AnalysisConfig,
    enhanced_graph: EnhancedCallGraph,
}

impl EnhancedCallGraphBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: AnalysisConfig::default(),
            enhanced_graph: EnhancedCallGraph {
                base_graph: CallGraph::new(),
                trait_registry: TraitRegistry::new(),
                function_pointer_tracker: FunctionPointerTracker::new(),
                framework_patterns: FrameworkPatternDetector::new(),
                cross_module_tracker: CrossModuleTracker::new(),
            },
        }
    }

    /// Create a builder with custom configuration
    pub fn with_config(config: AnalysisConfig) -> Self {
        Self {
            config,
            enhanced_graph: EnhancedCallGraph {
                base_graph: CallGraph::new(),
                trait_registry: TraitRegistry::new(),
                function_pointer_tracker: FunctionPointerTracker::new(),
                framework_patterns: FrameworkPatternDetector::new(),
                cross_module_tracker: CrossModuleTracker::new(),
            },
        }
    }

    /// Start with an existing base call graph
    pub fn from_base_graph(base_graph: CallGraph) -> Self {
        Self {
            config: AnalysisConfig::default(),
            enhanced_graph: EnhancedCallGraph {
                base_graph,
                trait_registry: TraitRegistry::new(),
                function_pointer_tracker: FunctionPointerTracker::new(),
                framework_patterns: FrameworkPatternDetector::new(),
                cross_module_tracker: CrossModuleTracker::new(),
            },
        }
    }

    /// Phase 1: Analyze basic function calls (uses existing functionality)
    pub fn analyze_basic_calls(&mut self, _file_path: &Path, _ast: &File) -> Result<&mut Self> {
        // This would integrate with existing call graph extraction
        // For now, we assume the base graph is already populated
        Ok(self)
    }

    /// Phase 2: Analyze trait implementations and method dispatch
    pub fn analyze_trait_dispatch(&mut self, file_path: &Path, ast: &File) -> Result<&mut Self> {
        if self.config.enable_trait_analysis {
            self.enhanced_graph
                .trait_registry
                .analyze_file(file_path, ast)?;
            self.resolve_trait_method_calls()?;
            self.mark_visit_trait_methods()?;
        }
        Ok(self)
    }

    /// Phase 3: Analyze function pointers and closures
    pub fn analyze_function_pointers(&mut self, file_path: &Path, ast: &File) -> Result<&mut Self> {
        if self.config.enable_function_pointer_tracking {
            self.enhanced_graph
                .function_pointer_tracker
                .analyze_file(file_path, ast)?;
            self.resolve_function_pointer_calls()?;
        }
        Ok(self)
    }

    /// Phase 4: Detect framework patterns
    pub fn analyze_framework_patterns(
        &mut self,
        file_path: &Path,
        ast: &File,
    ) -> Result<&mut Self> {
        if self.config.enable_framework_patterns {
            self.enhanced_graph
                .framework_patterns
                .analyze_file(file_path, ast)?;
            self.apply_framework_exclusions()?;
        }
        Ok(self)
    }

    /// Phase 5: Analyze cross-module dependencies
    pub fn analyze_cross_module(
        &mut self,
        workspace_files: &[(PathBuf, File)],
    ) -> Result<&mut Self> {
        if self.config.enable_cross_module_analysis {
            self.enhanced_graph
                .cross_module_tracker
                .analyze_workspace(workspace_files)?;
            self.resolve_cross_module_calls()?;
        }
        Ok(self)
    }

    /// Complete the analysis and return the enhanced call graph
    pub fn build(self) -> EnhancedCallGraph {
        self.enhanced_graph
    }

    /// Resolve trait method calls to their implementations
    fn resolve_trait_method_calls(&mut self) -> Result<()> {
        let trait_calls = self
            .enhanced_graph
            .trait_registry
            .get_unresolved_trait_calls();

        for trait_call in trait_calls {
            if let Some(implementations) = self
                .enhanced_graph
                .trait_registry
                .find_implementations(&trait_call.trait_name)
            {
                for implementation in implementations {
                    // Add call edges from trait call to each implementation
                    let call = FunctionCall {
                        caller: trait_call.caller.clone(),
                        callee: implementation.method_id.clone(),
                        call_type: CallType::Delegate, // Trait dispatch is delegation
                    };
                    self.enhanced_graph.base_graph.add_call(call);
                }
            }
        }

        Ok(())
    }

    /// Resolve function pointer and closure calls
    fn resolve_function_pointer_calls(&mut self) -> Result<()> {
        let pointer_calls = self
            .enhanced_graph
            .function_pointer_tracker
            .get_function_pointer_calls();

        for pointer_call in pointer_calls {
            if let Some(target_functions) = self
                .enhanced_graph
                .function_pointer_tracker
                .resolve_pointer_targets(&pointer_call.pointer_id)
            {
                for target in target_functions {
                    let call = FunctionCall {
                        caller: pointer_call.caller.clone(),
                        callee: target,
                        call_type: CallType::Callback, // Function pointers are callbacks
                    };
                    self.enhanced_graph.base_graph.add_call(call);
                }
            }
        }

        Ok(())
    }

    /// Apply framework pattern exclusions to reduce false positives
    fn apply_framework_exclusions(&mut self) -> Result<()> {
        let patterns = self
            .enhanced_graph
            .framework_patterns
            .get_detected_patterns();

        for pattern in patterns {
            match pattern.pattern_type {
                PatternType::TestFunction => {
                    // Mark as test function in base graph
                    // Test functions are entry points and shouldn't be marked as dead code
                    if let Some(_func_id) = &pattern.function_id {
                        // This would require extending the base CallGraph to support marking
                        // functions as framework-managed
                    }
                }
                PatternType::WebHandler | PatternType::EventHandler => {
                    // These are external entry points
                    if let Some(_func_id) = &pattern.function_id {
                        // Mark as entry point to prevent dead code detection
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Resolve cross-module function calls
    fn resolve_cross_module_calls(&mut self) -> Result<()> {
        let cross_module_calls = self
            .enhanced_graph
            .cross_module_tracker
            .get_cross_module_calls();

        for cross_call in cross_module_calls {
            if let Some(target_function) = self
                .enhanced_graph
                .cross_module_tracker
                .resolve_module_call(&cross_call.module_path, &cross_call.function_name)
            {
                let call = FunctionCall {
                    caller: cross_call.caller.clone(),
                    callee: target_function,
                    call_type: CallType::Direct,
                };
                self.enhanced_graph.base_graph.add_call(call);
            }
        }

        Ok(())
    }

    /// Mark Visit trait methods as framework-managed
    fn mark_visit_trait_methods(&mut self) -> Result<()> {
        let visit_methods = self.enhanced_graph.trait_registry.get_visit_trait_methods();

        for method_id in visit_methods {
            // Add Visit trait methods to framework patterns
            self.enhanced_graph
                .framework_patterns
                .add_visit_trait_function(method_id);
        }

        Ok(())
    }
}

impl EnhancedCallGraph {
    /// Create a new enhanced call graph
    pub fn new() -> Self {
        Self {
            base_graph: CallGraph::new(),
            trait_registry: TraitRegistry::new(),
            function_pointer_tracker: FunctionPointerTracker::new(),
            framework_patterns: FrameworkPatternDetector::new(),
            cross_module_tracker: CrossModuleTracker::new(),
        }
    }

    /// Determine if a framework pattern represents a live function
    fn is_live_pattern(pattern_type: &PatternType) -> bool {
        matches!(
            pattern_type,
            PatternType::TestFunction
                | PatternType::WebHandler
                | PatternType::EventHandler
                | PatternType::MacroCallback
                | PatternType::VisitTrait
        )
    }

    /// Collect framework-managed live functions
    fn collect_framework_functions(&self) -> impl Iterator<Item = FunctionId> + '_ {
        self.framework_patterns
            .get_detected_patterns()
            .into_iter()
            .filter_map(|pattern| {
                pattern.function_id.as_ref().and_then(|func_id| {
                    if Self::is_live_pattern(&pattern.pattern_type) {
                        Some(func_id.clone())
                    } else {
                        None
                    }
                })
            })
    }

    /// Collect all initially live functions (entry points, tests, framework, public API)
    fn collect_initial_live_functions(&self) -> HashSet<FunctionId> {
        let mut live_functions = HashSet::new();

        // Entry points
        live_functions.extend(self.base_graph.find_entry_points());

        // Test functions
        live_functions.extend(self.base_graph.find_test_functions());

        // Framework-managed functions
        live_functions.extend(self.collect_framework_functions());

        // Public API functions
        live_functions.extend(
            self.cross_module_tracker
                .get_public_apis()
                .into_iter()
                .map(|api| api.function_id),
        );

        live_functions
    }

    /// Get functions that are definitely used (not dead code)
    pub fn get_live_functions(&self) -> HashSet<FunctionId> {
        let mut live_functions = self.collect_initial_live_functions();

        // Add all functions reachable from live functions
        let mut to_visit: Vec<FunctionId> = live_functions.iter().cloned().collect();

        while let Some(current) = to_visit.pop() {
            for callee in self.base_graph.get_callees(&current) {
                if !live_functions.contains(&callee) {
                    live_functions.insert(callee.clone());
                    to_visit.push(callee);
                }
            }
        }

        live_functions
    }

    /// Identify potential dead code with reduced false positives
    pub fn get_potential_dead_code(&self) -> HashSet<FunctionId> {
        let live_functions = self.get_live_functions();
        let all_functions = self.base_graph.find_all_functions();

        all_functions
            .into_iter()
            .filter(|func_id| !live_functions.contains(func_id))
            .collect()
    }

    /// Get enhanced dead code analysis with context
    pub fn analyze_dead_code(&self) -> Vec<DeadCodeAnalysis> {
        let potential_dead_code = self.get_potential_dead_code();

        potential_dead_code
            .into_iter()
            .map(|func_id| {
                let confidence = self.calculate_dead_code_confidence(&func_id);
                let reasons = self.get_dead_code_reasons(&func_id);
                let false_positive_risks = self.get_false_positive_risks(&func_id);

                DeadCodeAnalysis {
                    function_id: func_id,
                    confidence,
                    reasons,
                    false_positive_risks,
                }
            })
            .collect()
    }

    /// Calculate confidence that a function is actually dead code
    fn calculate_dead_code_confidence(&self, func_id: &FunctionId) -> f64 {
        let mut confidence = 1.0;

        // Reduce confidence for functions that might be used by frameworks
        if self.framework_patterns.might_be_framework_managed(func_id) {
            confidence *= 0.3;
        }

        // Reduce confidence for public functions
        if self.cross_module_tracker.is_public_api(func_id) {
            confidence *= 0.2;
        }

        // Reduce confidence if function has trait implementations
        if self.trait_registry.has_trait_implementations(func_id) {
            confidence *= 0.4;
        }

        // Very low confidence for Visit trait methods (visitor pattern)
        if self.trait_registry.is_visit_trait_method(func_id) {
            confidence *= 0.1;
        }

        // Reduce confidence if function might be used through function pointers
        if self
            .function_pointer_tracker
            .might_be_called_through_pointer(func_id)
        {
            confidence *= 0.5;
        }

        confidence
    }

    /// Get reasons why a function is considered dead code
    fn get_dead_code_reasons(&self, func_id: &FunctionId) -> Vec<String> {
        let mut reasons = Vec::new();

        if self.base_graph.get_callers(func_id).is_empty() {
            reasons.push("No direct callers found".to_string());
        }

        if !self.base_graph.is_entry_point(func_id) {
            reasons.push("Not an entry point".to_string());
        }

        if !self.base_graph.is_test_function(func_id) {
            reasons.push("Not a test function".to_string());
        }

        reasons
    }

    /// Get potential false positive risks
    fn get_false_positive_risks(&self, func_id: &FunctionId) -> Vec<String> {
        let mut risks = Vec::new();

        if self.framework_patterns.might_be_framework_managed(func_id) {
            risks.push("Might be managed by framework".to_string());
        }

        if self.cross_module_tracker.is_public_api(func_id) {
            risks.push("Public API function".to_string());
        }

        if self.trait_registry.has_trait_implementations(func_id) {
            risks.push("Has trait implementations".to_string());
        }

        if self.trait_registry.is_visit_trait_method(func_id) {
            risks.push("Visit trait method (visitor pattern)".to_string());
        }

        if self
            .function_pointer_tracker
            .might_be_called_through_pointer(func_id)
        {
            risks.push("Might be called through function pointer".to_string());
        }

        risks
    }
}

/// Result of dead code analysis with enhanced context
#[derive(Debug, Clone)]
pub struct DeadCodeAnalysis {
    /// Function that might be dead code
    pub function_id: FunctionId,
    /// Confidence level (0.0 - 1.0) that this is actually dead code
    pub confidence: f64,
    /// Reasons why this function is considered dead code
    pub reasons: Vec<String>,
    /// Potential false positive risks
    pub false_positive_risks: Vec<String>,
}

impl Default for EnhancedCallGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for EnhancedCallGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
