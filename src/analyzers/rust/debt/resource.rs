//! Resource pattern analysis
//!
//! Detects resource management issues in Rust code.

use crate::core::DebtItem;
use crate::resource::{
    convert_resource_issue_to_debt_item, AsyncResourceDetector, DropDetector, ResourceDetector,
    UnboundedCollectionDetector,
};
use std::path::Path;

/// Analyze resource patterns in a file
pub fn analyze_resource_patterns(file: &syn::File, path: &Path) -> Vec<DebtItem> {
    let detectors: Vec<Box<dyn ResourceDetector>> = vec![
        Box::new(DropDetector::new()),
        Box::new(AsyncResourceDetector::new()),
        Box::new(UnboundedCollectionDetector::new()),
    ];

    let mut resource_items = Vec::new();

    for detector in detectors {
        let issues = detector.detect_issues(file, path);

        for issue in issues {
            let impact = detector.assess_resource_impact(&issue);
            let debt_item = convert_resource_issue_to_debt_item(issue, impact, path);
            resource_items.push(debt_item);
        }
    }

    resource_items
}
