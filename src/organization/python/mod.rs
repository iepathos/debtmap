use crate::organization::OrganizationAntiPattern;
use rustpython_parser::ast;
use std::path::Path;

mod simplified_implementation;
pub use simplified_implementation::SimplifiedPythonOrganizationDetector;

pub mod class_ownership;
pub mod domain_classifier;

pub use class_ownership::PythonClassAnalyzer;
pub use domain_classifier::classify_python_class_domain;

pub struct PythonOrganizationAnalyzer {
    detector: SimplifiedPythonOrganizationDetector,
}

impl PythonOrganizationAnalyzer {
    pub fn new() -> Self {
        Self {
            detector: SimplifiedPythonOrganizationDetector::new(),
        }
    }

    pub fn analyze(
        &self,
        module: &ast::Mod,
        path: &Path,
        source: &str,
    ) -> Vec<OrganizationAntiPattern> {
        self.detector.detect_patterns(module, path, source)
    }
}

impl Default for PythonOrganizationAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
