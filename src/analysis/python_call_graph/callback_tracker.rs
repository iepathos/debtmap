//! Callback Tracker Module
//!
//! This module provides comprehensive callback tracking with deferred resolution,
//! confidence scoring, and support for various callback patterns including
//! decorators, lambdas, and callbacks stored in data structures.

use crate::priority::call_graph::{CallGraph, CallType, FunctionCall, FunctionId};
use std::collections::HashMap;
use std::path::PathBuf;

/// Callback type classification
#[derive(Debug, Clone, PartialEq)]
pub enum CallbackType {
    EventBinding,
    RouteDecorator,
    SignalConnection,
    DirectAssignment,
    DictionaryStorage,
    ListStorage,
    Lambda,
    Partial,
}

/// Context information for callback resolution
#[derive(Debug, Clone)]
pub struct CallbackContext {
    pub current_class: Option<String>,
    pub current_function: Option<String>,
    pub scope_variables: HashMap<String, String>, // var_name -> function_name
}

/// A callback that needs deferred resolution
#[derive(Debug, Clone)]
pub struct PendingCallback {
    pub callback_expr: String,
    pub registration_point: Location,
    pub registration_type: CallbackType,
    pub context: CallbackContext,
    pub target_hint: Option<String>, // Hint about what function this might call
}

/// Location information for callback registration
#[derive(Debug, Clone)]
pub struct Location {
    pub file: PathBuf,
    pub line: usize,
    pub caller_function: Option<String>,
}

/// A resolved callback reference with confidence
#[derive(Debug, Clone)]
pub struct CallbackRef {
    pub target_function: FunctionId,
    pub confidence: f32,
}

/// Resolved callback with its registration information
#[derive(Debug, Clone)]
pub struct ResolvedCallback {
    pub callback_ref: CallbackRef,
    pub registration_point: Location,
}

/// Resolution result for a callback
#[derive(Debug)]
pub struct ResolutionResult {
    pub resolved_callbacks: Vec<ResolvedCallback>,
    pub unresolved: Vec<String>, // Expressions that couldn't be resolved
}

/// Main callback tracker with deferred resolution
pub struct CallbackTracker {
    pending_callbacks: Vec<PendingCallback>,
    callback_storage: HashMap<String, Vec<CallbackRef>>,
    decorator_targets: HashMap<String, Vec<String>>, // decorator -> functions it decorates
}

impl CallbackTracker {
    pub fn new() -> Self {
        Self {
            pending_callbacks: Vec::new(),
            callback_storage: HashMap::new(),
            decorator_targets: HashMap::new(),
        }
    }

    /// Track a callback for deferred resolution
    pub fn track_callback(&mut self, pending: PendingCallback) {
        self.pending_callbacks.push(pending);
    }

    /// Track a decorator application (e.g., @app.route)
    pub fn track_decorator(&mut self, decorator_name: String, target_function: String) {
        self.decorator_targets
            .entry(decorator_name)
            .or_default()
            .push(target_function);
    }

    /// Track callback storage in a variable/dict/list
    pub fn track_storage(&mut self, storage_key: String, callback_ref: CallbackRef) {
        self.callback_storage
            .entry(storage_key)
            .or_default()
            .push(callback_ref);
    }

    /// Resolve all pending callbacks against known functions
    pub fn resolve_callbacks(
        &self,
        known_functions: &HashMap<String, FunctionId>,
    ) -> ResolutionResult {
        let mut resolved_callbacks = Vec::new();
        let mut unresolved = Vec::new();

        for pending in &self.pending_callbacks {
            match self.resolve_single_callback(pending, known_functions) {
                Some(refs) => {
                    // Pair each callback_ref with its registration point
                    for callback_ref in refs {
                        resolved_callbacks.push(ResolvedCallback {
                            callback_ref,
                            registration_point: pending.registration_point.clone(),
                        });
                    }
                }
                None => unresolved.push(pending.callback_expr.clone()),
            }
        }

        ResolutionResult {
            resolved_callbacks,
            unresolved,
        }
    }

    /// Resolve a single pending callback
    fn resolve_single_callback(
        &self,
        pending: &PendingCallback,
        known_functions: &HashMap<String, FunctionId>,
    ) -> Option<Vec<CallbackRef>> {
        let confidence = self.get_callback_confidence(pending);

        // Try to resolve based on callback type
        match &pending.registration_type {
            CallbackType::EventBinding | CallbackType::SignalConnection => {
                self.resolve_method_reference(pending, known_functions, confidence)
            }
            CallbackType::RouteDecorator => {
                self.resolve_decorator_callback(pending, known_functions, confidence)
            }
            CallbackType::DirectAssignment => {
                self.resolve_direct_assignment(pending, known_functions, confidence)
            }
            CallbackType::DictionaryStorage | CallbackType::ListStorage => {
                self.resolve_storage_callback(pending, known_functions, confidence)
            }
            CallbackType::Lambda => {
                // Lambdas are difficult to resolve statically
                // Return None for now, could be enhanced with flow analysis
                None
            }
            CallbackType::Partial => {
                self.resolve_partial_callback(pending, known_functions, confidence)
            }
        }
    }

    /// Resolve method reference (self.method, cls.method)
    fn resolve_method_reference(
        &self,
        pending: &PendingCallback,
        known_functions: &HashMap<String, FunctionId>,
        confidence: f32,
    ) -> Option<Vec<CallbackRef>> {
        // Extract method name from expr like "self.on_click"
        let method_name = pending
            .callback_expr
            .strip_prefix("self.")
            .or_else(|| pending.callback_expr.strip_prefix("cls."))?;

        // Build full method name with class
        let full_name = if let Some(class_name) = &pending.context.current_class {
            format!("{}.{}", class_name, method_name)
        } else {
            method_name.to_string()
        };

        known_functions.get(&full_name).map(|func_id| {
            vec![CallbackRef {
                target_function: func_id.clone(),
                confidence,
            }]
        })
    }

    /// Resolve decorator-based callback
    fn resolve_decorator_callback(
        &self,
        pending: &PendingCallback,
        known_functions: &HashMap<String, FunctionId>,
        confidence: f32,
    ) -> Option<Vec<CallbackRef>> {
        // For decorators, the callback_expr is the decorator name
        // and we need to find all functions it decorates
        self.decorator_targets
            .get(&pending.callback_expr)
            .map(|targets| {
                targets
                    .iter()
                    .filter_map(|target| {
                        known_functions.get(target).map(|func_id| CallbackRef {
                            target_function: func_id.clone(),
                            confidence,
                        })
                    })
                    .collect()
            })
    }

    /// Resolve direct assignment (func_name as argument)
    fn resolve_direct_assignment(
        &self,
        pending: &PendingCallback,
        known_functions: &HashMap<String, FunctionId>,
        confidence: f32,
    ) -> Option<Vec<CallbackRef>> {
        // Check if it's a self.method or cls.method reference
        if let Some(method_name) = pending.callback_expr.strip_prefix("self.") {
            if let Some(class_name) = &pending.context.current_class {
                let full_name = format!("{}.{}", class_name, method_name);
                if let Some(func_id) = known_functions.get(&full_name) {
                    return Some(vec![CallbackRef {
                        target_function: func_id.clone(),
                        confidence,
                    }]);
                }
            }
        }

        // Try as-is first
        if let Some(func_id) = known_functions.get(&pending.callback_expr) {
            return Some(vec![CallbackRef {
                target_function: func_id.clone(),
                confidence,
            }]);
        }

        // Try as nested function
        if let Some(parent) = &pending.context.current_function {
            let nested_name = format!("{}.{}", parent, pending.callback_expr);
            if let Some(func_id) = known_functions.get(&nested_name) {
                return Some(vec![CallbackRef {
                    target_function: func_id.clone(),
                    confidence: confidence * 0.9, // Slightly lower confidence
                }]);
            }
        }

        None
    }

    /// Resolve callback from storage (dict/list)
    fn resolve_storage_callback(
        &self,
        pending: &PendingCallback,
        _known_functions: &HashMap<String, FunctionId>,
        _confidence: f32,
    ) -> Option<Vec<CallbackRef>> {
        // Look up in callback_storage
        self.callback_storage.get(&pending.callback_expr).cloned()
    }

    /// Resolve partial function callback
    fn resolve_partial_callback(
        &self,
        pending: &PendingCallback,
        known_functions: &HashMap<String, FunctionId>,
        confidence: f32,
    ) -> Option<Vec<CallbackRef>> {
        // For functools.partial, the callback_expr is already the function being partially applied
        // No need to extract - just resolve it directly
        self.resolve_direct_assignment(
            pending,
            known_functions,
            confidence * 0.85, // Lower confidence for partial
        )
    }

    /// Calculate confidence score for a callback
    pub fn get_callback_confidence(&self, callback: &PendingCallback) -> f32 {
        let mut confidence: f32 = 1.0;

        // Adjust based on callback type
        match callback.registration_type {
            CallbackType::EventBinding | CallbackType::DirectAssignment => {
                // High confidence for direct references
                confidence *= 0.95;
            }
            CallbackType::RouteDecorator => {
                // High confidence for decorators
                confidence *= 0.90;
            }
            CallbackType::SignalConnection => {
                // Medium-high confidence
                confidence *= 0.85;
            }
            CallbackType::DictionaryStorage | CallbackType::ListStorage => {
                // Medium confidence - storage can be complex
                confidence *= 0.75;
            }
            CallbackType::Partial => {
                // Medium confidence
                confidence *= 0.70;
            }
            CallbackType::Lambda => {
                // Low confidence - hard to resolve
                confidence *= 0.50;
            }
        }

        // Adjust based on context availability
        if callback.context.current_class.is_none() && callback.context.current_function.is_none() {
            confidence *= 0.8; // Lower confidence without context
        }

        // Boost if we have a hint
        if callback.target_hint.is_some() {
            confidence *= 1.1;
        }

        confidence.min(1.0)
    }

    /// Add resolved callbacks to the call graph
    pub fn add_to_call_graph(&self, resolution: &ResolutionResult, call_graph: &mut CallGraph) {
        for resolved in &resolution.resolved_callbacks {
            if let Some(caller_func) = &resolved.registration_point.caller_function {
                // Look up the caller in the call graph to get the correct line number
                let caller_id = call_graph
                    .get_all_functions()
                    .find(|f| f.name == *caller_func)
                    .cloned()
                    .unwrap_or_else(|| FunctionId {
                        name: caller_func.clone(),
                        file: resolved.registration_point.file.clone(),
                        line: resolved.registration_point.line,
                    });

                let call = FunctionCall {
                    caller: caller_id,
                    callee: resolved.callback_ref.target_function.clone(),
                    call_type: if resolved.callback_ref.confidence > 0.8 {
                        CallType::Direct
                    } else {
                        CallType::Callback
                    },
                };

                call_graph.add_call(call);
            }
        }
    }

    /// Get all pending callbacks (for debugging/reporting)
    pub fn get_pending_callbacks(&self) -> &[PendingCallback] {
        &self.pending_callbacks
    }

    /// Get decorator targets
    pub fn get_decorator_targets(&self) -> &HashMap<String, Vec<String>> {
        &self.decorator_targets
    }
}

impl Default for CallbackTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_reference_resolution() {
        let mut tracker = CallbackTracker::new();
        let mut known_functions = HashMap::new();

        let func_id = FunctionId {
            name: "MyClass.on_click".to_string(),
            file: PathBuf::from("test.py"),
            line: 10,
        };
        known_functions.insert("MyClass.on_click".to_string(), func_id.clone());

        let pending = PendingCallback {
            callback_expr: "self.on_click".to_string(),
            registration_point: Location {
                file: PathBuf::from("test.py"),
                line: 20,
                caller_function: Some("MyClass.setup".to_string()),
            },
            registration_type: CallbackType::EventBinding,
            context: CallbackContext {
                current_class: Some("MyClass".to_string()),
                current_function: Some("MyClass.setup".to_string()),
                scope_variables: HashMap::new(),
            },
            target_hint: None,
        };

        tracker.track_callback(pending);
        let result = tracker.resolve_callbacks(&known_functions);

        assert_eq!(result.resolved_callbacks.len(), 1);
        assert_eq!(
            result.resolved_callbacks[0]
                .callback_ref
                .target_function
                .name,
            "MyClass.on_click"
        );
        assert!(result.resolved_callbacks[0].callback_ref.confidence > 0.8);
    }

    #[test]
    fn test_nested_function_resolution() {
        let mut tracker = CallbackTracker::new();
        let mut known_functions = HashMap::new();

        let func_id = FunctionId {
            name: "outer.inner".to_string(),
            file: PathBuf::from("test.py"),
            line: 15,
        };
        known_functions.insert("outer.inner".to_string(), func_id.clone());

        let pending = PendingCallback {
            callback_expr: "inner".to_string(),
            registration_point: Location {
                file: PathBuf::from("test.py"),
                line: 20,
                caller_function: Some("outer".to_string()),
            },
            registration_type: CallbackType::DirectAssignment,
            context: CallbackContext {
                current_class: None,
                current_function: Some("outer".to_string()),
                scope_variables: HashMap::new(),
            },
            target_hint: None,
        };

        tracker.track_callback(pending);
        let result = tracker.resolve_callbacks(&known_functions);

        assert_eq!(result.resolved_callbacks.len(), 1);
        assert_eq!(
            result.resolved_callbacks[0]
                .callback_ref
                .target_function
                .name,
            "outer.inner"
        );
    }

    #[test]
    fn test_confidence_scoring() {
        let tracker = CallbackTracker::new();

        let high_confidence = PendingCallback {
            callback_expr: "self.handler".to_string(),
            registration_point: Location {
                file: PathBuf::from("test.py"),
                line: 10,
                caller_function: Some("MyClass.setup".to_string()),
            },
            registration_type: CallbackType::EventBinding,
            context: CallbackContext {
                current_class: Some("MyClass".to_string()),
                current_function: Some("MyClass.setup".to_string()),
                scope_variables: HashMap::new(),
            },
            target_hint: None,
        };

        let low_confidence = PendingCallback {
            callback_expr: "lambda x: x + 1".to_string(),
            registration_point: Location {
                file: PathBuf::from("test.py"),
                line: 10,
                caller_function: None,
            },
            registration_type: CallbackType::Lambda,
            context: CallbackContext {
                current_class: None,
                current_function: None,
                scope_variables: HashMap::new(),
            },
            target_hint: None,
        };

        let high_score = tracker.get_callback_confidence(&high_confidence);
        let low_score = tracker.get_callback_confidence(&low_confidence);

        assert!(high_score > 0.8);
        assert!(low_score < 0.6);
        assert!(high_score > low_score);
    }

    #[test]
    fn test_decorator_tracking() {
        let mut tracker = CallbackTracker::new();

        tracker.track_decorator("app.route".to_string(), "index".to_string());
        tracker.track_decorator("app.route".to_string(), "about".to_string());

        let targets = tracker.get_decorator_targets();
        assert_eq!(targets.get("app.route").unwrap().len(), 2);
        assert!(targets
            .get("app.route")
            .unwrap()
            .contains(&"index".to_string()));
    }
}
