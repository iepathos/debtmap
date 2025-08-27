use crate::core::{DebtItem, DebtType};
use im::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionId {
    pub file: PathBuf,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionDebtProfile {
    pub function_id: FunctionId,
    pub organization_issues: Vec<DebtItem>,
    pub testing_issues: Vec<DebtItem>,
    pub resource_issues: Vec<DebtItem>,
    pub duplication_issues: Vec<DebtItem>,
}

impl FunctionDebtProfile {
    pub fn new(function_id: FunctionId) -> Self {
        Self {
            function_id,
            organization_issues: Vec::new(),
            testing_issues: Vec::new(),
            resource_issues: Vec::new(),
            duplication_issues: Vec::new(),
        }
    }

    pub fn add_debt_item(&mut self, item: DebtItem) {
        match categorize_debt_type(&item.debt_type) {
            DebtCategory::Organization => self.organization_issues.push(item),
            DebtCategory::Testing => self.testing_issues.push(item),
            DebtCategory::Resource => self.resource_issues.push(item),
            DebtCategory::Duplication => self.duplication_issues.push(item),
        }
    }

    pub fn total_issues(&self) -> usize {
        self.organization_issues.len()
            + self.testing_issues.len()
            + self.resource_issues.len()
            + self.duplication_issues.len()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DebtCategory {
    Organization,
    Testing,
    Resource,
    Duplication,
}

impl DebtCategory {
    pub fn severity_weight(&self) -> f64 {
        match self {
            DebtCategory::Resource => 2.5, // High priority (error handling, leaks)
            DebtCategory::Organization => 1.5, // Medium priority
            DebtCategory::Testing => 1.0,  // Lower priority
            DebtCategory::Duplication => 1.2, // Medium-low priority
        }
    }
}

pub fn categorize_debt_type(debt_type: &DebtType) -> DebtCategory {
    match debt_type {
        // Complexity is an organization/maintainability issue
        DebtType::Complexity => DebtCategory::Organization,

        // Organization issues
        DebtType::Todo | DebtType::Fixme => DebtCategory::Organization,
        DebtType::CodeOrganization => DebtCategory::Organization,
        DebtType::CodeSmell => DebtCategory::Organization,
        DebtType::Dependency => DebtCategory::Organization,

        // Testing issues
        DebtType::TestComplexity | DebtType::TestTodo | DebtType::TestDuplication => {
            DebtCategory::Testing
        }
        DebtType::TestQuality => DebtCategory::Testing,

        // Resource management issues
        DebtType::ErrorSwallowing => DebtCategory::Resource,
        DebtType::ResourceManagement => DebtCategory::Resource,

        // Duplication issues
        DebtType::Duplication => DebtCategory::Duplication,
    }
}

#[derive(Debug, Clone, Default)]
pub struct DebtAggregator {
    profiles: HashMap<FunctionId, FunctionDebtProfile>,
    debt_index: HashMap<PathBuf, Vec<DebtItem>>,
}

impl DebtAggregator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn aggregate_debt(
        &mut self,
        items: Vec<DebtItem>,
        functions: &[(FunctionId, usize, usize)],
    ) {
        // First, index all debt items by file
        for item in items {
            let file = item.file.clone();
            self.debt_index.entry(file).or_default().push(item);
        }

        // Now map debt items to functions based on line ranges
        for (func_id, _start, _end) in functions {
            let mut profile = FunctionDebtProfile::new(func_id.clone());

            if let Some(file_debts) = self.debt_index.get(&func_id.file) {
                for debt_item in file_debts {
                    // Check if the debt item falls within this function's line range
                    if debt_item.line >= func_id.start_line && debt_item.line <= func_id.end_line {
                        profile.add_debt_item(debt_item.clone());
                    }
                }
            }

            self.profiles.insert(func_id.clone(), profile);
        }
    }

    pub fn get_profile(&self, func_id: &FunctionId) -> Option<&FunctionDebtProfile> {
        self.profiles.get(func_id)
    }

    pub fn calculate_debt_scores(&self, func_id: &FunctionId) -> DebtScores {
        self.profiles
            .get(func_id)
            .map(|profile| DebtScores {
                organization: calculate_category_score(
                    &profile.organization_issues,
                    DebtCategory::Organization,
                ),
                testing: calculate_category_score(&profile.testing_issues, DebtCategory::Testing),
                resource: calculate_category_score(
                    &profile.resource_issues,
                    DebtCategory::Resource,
                ),
                duplication: calculate_category_score(
                    &profile.duplication_issues,
                    DebtCategory::Duplication,
                ),
            })
            .unwrap_or_default()
    }
}

fn calculate_category_score(issues: &[DebtItem], category: DebtCategory) -> f64 {
    use crate::core::Priority;

    if issues.is_empty() {
        return 0.0;
    }

    let base_score = issues.len() as f64;
    let severity_weight = category.severity_weight();

    // Calculate weighted score based on issue priorities
    let priority_sum: f64 = issues
        .iter()
        .map(|item| match item.priority {
            Priority::Critical => 3.0,
            Priority::High => 2.0,
            Priority::Medium => 1.0,
            Priority::Low => 0.5,
        })
        .sum();

    // Normalize and apply category weight
    let score = (priority_sum / issues.len() as f64) * severity_weight * base_score.sqrt();
    score.min(10.0) // Cap at 10.0
}

#[derive(Debug, Clone, Default)]
pub struct DebtScores {
    pub organization: f64,
    pub testing: f64,
    pub resource: f64,
    pub duplication: f64,
}

impl DebtScores {
    pub fn total(&self) -> f64 {
        self.organization + self.testing + self.resource + self.duplication
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_debt_categorization() {
        assert_eq!(
            categorize_debt_type(&DebtType::ErrorSwallowing),
            DebtCategory::Resource
        );
        assert_eq!(
            categorize_debt_type(&DebtType::Todo),
            DebtCategory::Organization
        );
        assert_eq!(
            categorize_debt_type(&DebtType::TestQuality),
            DebtCategory::Testing
        );
        assert_eq!(
            categorize_debt_type(&DebtType::Complexity),
            DebtCategory::Organization
        );
        assert_eq!(
            categorize_debt_type(&DebtType::Duplication),
            DebtCategory::Duplication
        );
    }

    #[test]
    fn test_debt_aggregation() {
        let mut aggregator = DebtAggregator::new();

        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "test_func".to_string(),
            start_line: 10,
            end_line: 20,
        };

        let debt_items = vec![
            DebtItem {
                id: "2".to_string(),
                file: PathBuf::from("test.rs"),
                line: 18,
                column: None,
                debt_type: DebtType::Todo,
                message: "TODO: fix this".to_string(),
                priority: crate::core::Priority::Low,
                context: None,
            },
            // This one should not be included (outside function range)
            DebtItem {
                id: "3".to_string(),
                file: PathBuf::from("test.rs"),
                line: 25,
                column: None,
                debt_type: DebtType::ErrorSwallowing,
                message: "Error swallowed".to_string(),
                priority: crate::core::Priority::Medium,
                context: None,
            },
        ];

        aggregator.aggregate_debt(debt_items, &[(func_id.clone(), 10, 20)]);

        let profile = aggregator.get_profile(&func_id).unwrap();
        assert_eq!(profile.organization_issues.len(), 1);
        assert_eq!(profile.resource_issues.len(), 0); // Outside range
        assert_eq!(profile.total_issues(), 1);
    }

    #[test]
    fn test_debt_score_calculation() {
        let mut aggregator = DebtAggregator::new();

        let func_id = FunctionId {
            file: PathBuf::from("test.rs"),
            name: "critical_func".to_string(),
            start_line: 1,
            end_line: 50,
        };

        let debt_items = vec![
            DebtItem {
                id: "4".to_string(),
                file: PathBuf::from("test.rs"),
                line: 10,
                column: None,
                debt_type: DebtType::Todo,
                message: "Critical todo issue".to_string(),
                priority: crate::core::Priority::Critical,
                context: None,
            },
            DebtItem {
                id: "5".to_string(),
                file: PathBuf::from("test.rs"),
                line: 20,
                column: None,
                debt_type: DebtType::Todo,
                message: "High todo issue".to_string(),
                priority: crate::core::Priority::High,
                context: None,
            },
        ];

        aggregator.aggregate_debt(debt_items, &[(func_id.clone(), 1, 50)]);

        let scores = aggregator.calculate_debt_scores(&func_id);
        assert!(scores.organization > 0.0);
        assert!(scores.organization <= 10.0);
    }
}
