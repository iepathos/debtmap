//! Core type definitions for Python type tracking
//!
//! This module contains the fundamental type definitions used throughout the
//! type tracking system, including type representations, function signatures,
//! class information, and scope management.

use crate::priority::call_graph::FunctionId;
use std::collections::{HashMap, HashSet};

/// Python type representation for tracking
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PythonType {
    /// Class type (e.g., `MyClass`)
    Class(String),
    /// Instance of a class (e.g., `MyClass()`)
    Instance(String),
    /// Function or method
    Function(FunctionSignature),
    /// Module
    Module(String),
    /// Union of multiple possible types
    Union(Vec<PythonType>),
    /// Built-in type
    BuiltIn(String),
    /// Unknown type
    Unknown,
}

/// Function signature information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSignature {
    pub name: String,
    pub params: Vec<String>,
    pub return_type: Option<Box<PythonType>>,
}

/// Class information including hierarchy and members
#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub bases: Vec<String>,
    pub methods: HashMap<String, FunctionId>,
    pub attributes: HashMap<String, PythonType>,
    pub static_methods: HashSet<String>,
    pub class_methods: HashSet<String>,
    pub properties: HashSet<String>,
}

/// Scope information for tracking variables
#[derive(Debug, Clone)]
pub struct Scope {
    pub variables: HashMap<String, PythonType>,
    pub parent: Option<Box<Scope>>,
}

impl Scope {
    pub(crate) fn new() -> Self {
        Self {
            variables: HashMap::new(),
            parent: None,
        }
    }

    pub(crate) fn with_parent(parent: Scope) -> Self {
        Self {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }

    pub(crate) fn lookup(&self, name: &str) -> Option<&PythonType> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }

    pub(crate) fn insert(&mut self, name: String, ty: PythonType) {
        self.variables.insert(name, ty);
    }
}
