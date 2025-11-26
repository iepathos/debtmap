//! Core trait definitions for clean module boundaries
//!
//! This module contains all shared trait definitions that establish
//! clear contracts between different parts of the codebase.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Core analyzer trait for all language analyzers
pub trait Analyzer: Send + Sync {
    /// The input type this analyzer processes
    type Input;
    /// The output type this analyzer produces
    type Output;

    /// Analyze the given input and produce output
    fn analyze(&self, input: Self::Input) -> Result<Self::Output>;

    /// Get the name of this analyzer for reporting
    fn name(&self) -> &str;
}

/// Trait for scoring technical debt items
pub trait Scorer: Send + Sync {
    /// The item type to be scored
    type Item;

    /// Calculate a score for the given item
    fn score(&self, item: &Self::Item) -> f64;

    /// Get a description of the scoring methodology
    fn methodology(&self) -> &str;
}

/// File system operations abstraction
pub trait FileSystem: Send + Sync {
    /// Read file contents as string
    fn read_file(&self, path: &Path) -> Result<String>;

    /// Write string contents to file
    fn write_file(&self, path: &Path, contents: &str) -> Result<()>;

    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// List all files matching a pattern
    fn glob(&self, pattern: &str) -> Result<Vec<std::path::PathBuf>>;
}

/// Parser trait for language-specific parsing
pub trait Parser: Send + Sync {
    /// The AST type produced by this parser
    type Ast;

    /// Parse source code into an AST
    fn parse(&self, source: &str, path: &Path) -> Result<Self::Ast>;

    /// Get the language this parser handles
    fn language(&self) -> &str;
}

/// Complexity calculator trait
pub trait ComplexityCalculator: Send + Sync {
    /// The input type for complexity calculation
    type Input;

    /// Calculate cyclomatic complexity
    fn cyclomatic_complexity(&self, input: &Self::Input) -> u32;

    /// Calculate cognitive complexity
    fn cognitive_complexity(&self, input: &Self::Input) -> u32;

    /// Calculate halstead metrics
    fn halstead_metrics(&self, input: &Self::Input) -> HalsteadMetrics;
}

/// Halstead metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalsteadMetrics {
    pub volume: f64,
    pub difficulty: f64,
    pub effort: f64,
    pub time: f64,
    pub bugs: f64,
}

/// Report formatter trait
pub trait Formatter: Send + Sync {
    /// The report type to format
    type Report;

    /// Format a report as string
    fn format(&self, report: &Self::Report) -> Result<String>;

    /// Get the output format name (json, markdown, etc)
    fn format_name(&self) -> &str;
}

/// Configuration provider trait
pub trait ConfigProvider: Send + Sync {
    /// Get configuration value by key
    fn get(&self, key: &str) -> Option<String>;

    /// Set configuration value
    fn set(&mut self, key: String, value: String);

    /// Load configuration from file
    fn load_from_file(&self, path: &Path) -> Result<()>;
}

/// Detector trait for pattern detection
pub trait Detector: Send + Sync {
    /// The context type this detector analyzes
    type Context;
    /// The detection result type
    type Detection;

    /// Detect patterns in the given context
    fn detect(&self, context: &Self::Context) -> Vec<Self::Detection>;

    /// Get detector confidence level
    fn confidence(&self) -> f64;
}

/// Priority calculator trait
pub trait PriorityCalculator: Send + Sync {
    /// The item type to prioritize
    type Item;

    /// Calculate priority score (0.0 to 1.0)
    fn calculate_priority(&self, item: &Self::Item) -> f64;

    /// Get factors that influenced the priority
    fn get_factors(&self, item: &Self::Item) -> Vec<PriorityFactor>;
}

/// Priority factor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityFactor {
    pub name: String,
    pub weight: f64,
    pub value: f64,
    pub description: String,
}

/// Refactoring opportunity detector
pub trait RefactoringDetector: Send + Sync {
    /// The code context to analyze
    type Context;
    /// The refactoring opportunity type
    type Opportunity;

    /// Detect refactoring opportunities
    fn detect_opportunities(&self, context: &Self::Context) -> Vec<Self::Opportunity>;

    /// Estimate effort for a refactoring
    fn estimate_effort(&self, opportunity: &Self::Opportunity) -> EffortEstimate;
}

/// Effort estimate for refactoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffortEstimate {
    pub hours: f64,
    pub complexity: String,
    pub risk_level: String,
    pub confidence: f64,
}

/// Test analyzer trait
pub trait TestAnalyzer: Send + Sync {
    /// The test suite type
    type TestSuite;

    /// Analyze test coverage
    fn analyze_coverage(&self, suite: &Self::TestSuite) -> CoverageReport;

    /// Detect test smells
    fn detect_test_smells(&self, suite: &Self::TestSuite) -> Vec<TestSmell>;
}

/// Coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub line_coverage: f64,
    pub branch_coverage: f64,
    pub function_coverage: f64,
    pub uncovered_lines: Vec<usize>,
}

/// Test smell detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSmell {
    pub smell_type: String,
    pub location: String,
    pub severity: String,
    pub suggestion: String,
}

/// Repository trait for data persistence
pub trait Repository<T>: Send + Sync {
    /// Save an entity
    fn save(&mut self, entity: T) -> Result<()>;

    /// Find entity by id
    fn find_by_id(&self, id: &str) -> Option<T>;

    /// Find all entities
    fn find_all(&self) -> Vec<T>;

    /// Delete entity by id
    fn delete(&mut self, id: &str) -> Result<()>;
}

/// Event publisher trait for decoupled communication
pub trait EventPublisher: Send + Sync {
    /// The event type to publish
    type Event;

    /// Publish an event
    fn publish(&self, event: Self::Event) -> Result<()>;
}

/// Event subscriber trait
pub trait EventSubscriber: Send + Sync {
    /// The event type to subscribe to
    type Event;

    /// Handle an event
    fn handle(&mut self, event: Self::Event) -> Result<()>;
}
