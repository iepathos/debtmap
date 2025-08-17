// JavaScript/TypeScript detector infrastructure
// Implements comprehensive debt detection using tree-sitter queries

pub mod organization;
pub mod performance;
pub mod resource;
pub mod security;
pub mod testing;

use crate::core::DebtItem;
use std::path::PathBuf;
use tree_sitter::{Node, Query};

/// Core types for JS/TS detector patterns
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub line: usize,
    pub column: Option<usize>,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
}

impl SourceLocation {
    pub fn from_node(node: Node) -> Self {
        let start = node.start_position();
        let end = node.end_position();

        SourceLocation {
            line: start.row + 1, // tree-sitter uses 0-based lines
            column: Some(start.column),
            end_line: Some(end.row + 1),
            end_column: Some(end.column),
        }
    }
}

/// Main visitor for JavaScript/TypeScript detection
pub struct JavaScriptDetectorVisitor {
    pub source_content: String,
    pub path: PathBuf,
    pub language: tree_sitter::Language,

    // Collected patterns
    pub performance_patterns: Vec<performance::PerformanceAntiPattern>,
    pub organization_patterns: Vec<organization::OrganizationAntiPattern>,
    pub security_vulnerabilities: Vec<security::SecurityVulnerability>,
    pub resource_issues: Vec<resource::ResourceManagementIssue>,
    pub testing_issues: Vec<testing::TestingAntiPattern>,
}

impl JavaScriptDetectorVisitor {
    pub fn new(source: String, path: PathBuf, language: tree_sitter::Language) -> Self {
        Self {
            source_content: source,
            path,
            language,
            performance_patterns: Vec::new(),
            organization_patterns: Vec::new(),
            security_vulnerabilities: Vec::new(),
            resource_issues: Vec::new(),
            testing_issues: Vec::new(),
        }
    }

    /// Visit the tree and run all detectors
    pub fn visit_tree(&mut self, tree: &tree_sitter::Tree) {
        let root_node = tree.root_node();

        // Run all detector modules
        self.detect_performance_patterns(root_node);
        self.detect_organization_patterns(root_node);
        self.detect_security_patterns(root_node);
        self.detect_resource_patterns(root_node);
        self.detect_testing_patterns(root_node);
    }

    fn detect_performance_patterns(&mut self, root: Node) {
        performance::detect_performance_patterns(
            root,
            &self.source_content,
            &self.language,
            &mut self.performance_patterns,
        );
    }

    fn detect_organization_patterns(&mut self, root: Node) {
        organization::detect_organization_patterns(
            root,
            &self.source_content,
            &self.language,
            &mut self.organization_patterns,
        );
    }

    fn detect_security_patterns(&mut self, root: Node) {
        security::detect_security_patterns(
            root,
            &self.source_content,
            &self.language,
            &mut self.security_vulnerabilities,
        );
    }

    fn detect_resource_patterns(&mut self, root: Node) {
        resource::detect_resource_patterns(
            root,
            &self.source_content,
            &self.language,
            &mut self.resource_issues,
        );
    }

    fn detect_testing_patterns(&mut self, root: Node) {
        testing::detect_testing_patterns(
            root,
            &self.source_content,
            &self.language,
            self.path.clone(),
            &mut self.testing_issues,
        );
    }

    /// Convert detected patterns to debt items
    pub fn to_debt_items(&self) -> Vec<DebtItem> {
        let mut items = Vec::new();

        // Convert performance patterns
        for pattern in &self.performance_patterns {
            items.push(pattern.to_debt_item(&self.path));
        }

        // Convert organization patterns
        for pattern in &self.organization_patterns {
            items.push(pattern.to_debt_item(&self.path));
        }

        // Convert security vulnerabilities
        for vuln in &self.security_vulnerabilities {
            items.push(vuln.to_debt_item(&self.path));
        }

        // Convert resource issues
        for issue in &self.resource_issues {
            items.push(issue.to_debt_item(&self.path));
        }

        // Convert testing issues
        for issue in &self.testing_issues {
            items.push(issue.to_debt_item(&self.path));
        }

        items
    }
}

/// Helper to get text from a node
pub fn get_node_text<'a>(node: Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

/// Helper to create a query safely
pub fn create_query(language: &tree_sitter::Language, query_str: &str) -> Option<Query> {
    Query::new(language, query_str).ok()
}
