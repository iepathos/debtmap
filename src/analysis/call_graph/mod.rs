//! Rust-Specific Call Graph Analysis Module
//!
//! This module provides Rust-specific call graph analysis that addresses the false positives
//! in dead code detection by tracking trait implementations, function pointers, closures,
//! framework patterns, and cross-module dependencies specific to the Rust language.
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
pub mod effects;
mod framework_patterns;
mod function_pointer;
mod trait_registry;

pub use cross_module::{CrossModuleTracker, ModuleBoundary, PublicApiInfo};
pub use framework_patterns::{FrameworkPattern, FrameworkPatternDetector, PatternType};
pub use function_pointer::{ClosureInfo, FunctionPointerInfo, FunctionPointerTracker};
pub use trait_registry::{
    TraitImplementation, TraitMethod, TraitMethodCall, TraitMethodImplementation, TraitRegistry,
    TraitStatistics,
};

/// Rust-specific call graph that includes trait dispatch, function pointers, and framework patterns
#[derive(Debug, Clone)]
pub struct RustCallGraph {
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

/// Configuration for Rust-specific call graph analysis
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

/// Builder for Rust-specific call graph analysis
pub struct RustCallGraphBuilder {
    config: AnalysisConfig,
    enhanced_graph: RustCallGraph,
}

impl RustCallGraphBuilder {
    /// Create a new builder with default configuration
    pub fn new() -> Self {
        Self {
            config: AnalysisConfig::default(),
            enhanced_graph: RustCallGraph {
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
            enhanced_graph: RustCallGraph {
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
            enhanced_graph: RustCallGraph {
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

            // Initialize the trait resolver for enhanced resolution
            self.enhanced_graph.trait_registry.init_resolver();

            // NOTE: detect_common_trait_patterns should be called ONCE after all files
            // are processed, not once per file. See finalize_trait_analysis() method.

            self.resolve_trait_method_calls()?;
            self.mark_visit_trait_methods()?;
            self.resolve_trait_object_calls()?;
            self.resolve_generic_trait_bounds()?;
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

    /// Finalize trait analysis after all files have been processed
    /// This should be called ONCE after all per-file analysis is complete
    pub fn finalize_trait_analysis(&mut self) -> Result<()> {
        // Detect common trait patterns (Default, Clone, From, Into, constructors)
        self.enhanced_graph
            .trait_registry
            .detect_common_trait_patterns(&mut self.enhanced_graph.base_graph);

        // Resolve trait method calls after pattern detection
        let _resolved_count = self
            .enhanced_graph
            .trait_registry
            .resolve_trait_method_calls(&mut self.enhanced_graph.base_graph);

        Ok(())
    }

    /// Complete the analysis and return the Rust-specific call graph
    pub fn build(self) -> RustCallGraph {
        self.enhanced_graph
    }

    /// Resolve trait method calls to their implementations
    fn resolve_trait_method_calls(&mut self) -> Result<()> {
        let trait_calls = self
            .enhanced_graph
            .trait_registry
            .get_unresolved_trait_calls();

        for trait_call in trait_calls {
            // Use enhanced resolution for better accuracy
            let resolved_impls = self
                .enhanced_graph
                .trait_registry
                .resolve_trait_call(&trait_call);

            for implementation in resolved_impls {
                // Add call edges from trait call to each implementation
                let call = FunctionCall {
                    caller: trait_call.caller.clone(),
                    callee: implementation,
                    call_type: CallType::Delegate, // Trait dispatch is delegation
                };
                self.enhanced_graph.base_graph.add_call(call);
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

    /// Resolve trait object calls (dyn Trait)
    fn resolve_trait_object_calls(&mut self) -> Result<()> {
        // Get the enhanced tracker to check for trait objects
        let tracker = self.enhanced_graph.trait_registry.get_enhanced_tracker();

        // For each trait with implementations, check if it's used as a trait object
        for (trait_name, _) in tracker.traits.iter() {
            let trait_object = crate::analyzers::trait_implementation_tracker::TraitObject {
                trait_name: trait_name.clone(),
                additional_bounds: im::Vector::new(),
                lifetime: None,
            };

            // Resolve all methods that could be called on this trait object
            let implementations = tracker.resolve_trait_object_call(
                &trait_object.trait_name,
                "", // Will be filled by actual method names
            );

            for impl_func in implementations {
                // Mark these functions as reachable through trait objects
                // This helps reduce false positives in dead code detection
                self.enhanced_graph
                    .base_graph
                    .mark_as_trait_dispatch(impl_func);
            }
        }

        Ok(())
    }

    /// Resolve generic trait bounds
    fn resolve_generic_trait_bounds(&mut self) -> Result<()> {
        // This would analyze generic functions with trait bounds
        // and resolve them to concrete implementations
        // For now, this is a placeholder for future enhancement
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

impl RustCallGraph {
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

    /// Classify function usage patterns and determine confidence adjustment factors
    fn get_confidence_adjustments(
        is_framework_managed: bool,
        is_public_api: bool,
        has_trait_implementations: bool,
        is_visit_trait_method: bool,
        might_be_called_through_pointer: bool,
    ) -> Vec<(bool, f64)> {
        vec![
            (is_framework_managed, 0.3),
            (is_public_api, 0.2),
            (has_trait_implementations, 0.4),
            (is_visit_trait_method, 0.1),
            (might_be_called_through_pointer, 0.5),
        ]
    }

    /// Apply confidence adjustments based on multiple factors
    fn apply_confidence_adjustments(
        base_confidence: f64,
        factors: impl Iterator<Item = (bool, f64)>,
    ) -> f64 {
        factors.fold(
            base_confidence,
            |conf, (applies, factor)| {
                if applies {
                    conf * factor
                } else {
                    conf
                }
            },
        )
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

    /// Get Rust-specific dead code analysis with context
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
        let adjustments = Self::get_confidence_adjustments(
            self.framework_patterns.might_be_framework_managed(func_id),
            self.cross_module_tracker.is_public_api(func_id),
            self.trait_registry.has_trait_implementations(func_id),
            self.trait_registry.is_visit_trait_method(func_id),
            self.function_pointer_tracker
                .might_be_called_through_pointer(func_id),
        );

        Self::apply_confidence_adjustments(1.0, adjustments.into_iter())
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

    /// Classify framework and pointer-related risks
    fn classify_framework_risks(
        framework_managed: bool,
        public_api: bool,
        function_pointer: bool,
    ) -> Vec<&'static str> {
        [
            (framework_managed, "Might be managed by framework"),
            (public_api, "Public API function"),
            (function_pointer, "Might be called through function pointer"),
        ]
        .into_iter()
        .filter_map(|(applies, risk)| if applies { Some(risk) } else { None })
        .collect()
    }

    /// Classify trait-related risks
    fn classify_trait_risks(has_implementations: bool, is_visit_method: bool) -> Vec<&'static str> {
        [
            (has_implementations, "Has trait implementations"),
            (is_visit_method, "Visit trait method (visitor pattern)"),
        ]
        .into_iter()
        .filter_map(|(applies, risk)| if applies { Some(risk) } else { None })
        .collect()
    }

    /// Get potential false positive risks
    fn get_false_positive_risks(&self, func_id: &FunctionId) -> Vec<String> {
        let framework_risks = Self::classify_framework_risks(
            self.framework_patterns.might_be_framework_managed(func_id),
            self.cross_module_tracker.is_public_api(func_id),
            self.function_pointer_tracker
                .might_be_called_through_pointer(func_id),
        );

        let trait_risks = Self::classify_trait_risks(
            self.trait_registry.has_trait_implementations(func_id),
            self.trait_registry.is_visit_trait_method(func_id),
        );

        framework_risks
            .into_iter()
            .chain(trait_risks)
            .map(|s| s.to_string())
            .collect()
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

impl Default for RustCallGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for RustCallGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
